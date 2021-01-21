//! Yew Agent for running code (in a web worker).

use std::rc::Rc;

use ella::builtin_functions;
use ella_parser::parser::Parser;
use ella_passes::resolve::Resolver;
use ella_value::BuiltinVars;
use ella_value::Value;
use ella_vm::codegen::Codegen;
use ella_vm::vm::{InterpretResult, Vm};
use enclose::enc;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use yew::worker::*;

#[wasm_bindgen(
    inline_js = "export function js_clock(a, b) { return new Date().valueOf() / 1000; }"
)]
extern "C" {
    fn js_clock() -> f64;
}

fn native_clock(_args: &mut [Value]) -> Value {
    let time = js_clock();
    Value::Number(time)
}

pub fn run(
    source: Rc<String>,
    report_output: Rc<impl Fn(String) + 'static>,
    report_errors: Rc<impl Fn(String)>,
) {
    let start = js_clock();

    let source = source.as_str().into();
    let mut builtin_vars = BuiltinVars::new();

    let output = Rc::new(RefCell::new(String::new()));
    let report_output = Rc::downgrade(&report_output);

    let native_println = Box::leak(Box::new(
        enc!((output, report_output) move |args: &mut [Value]| {
            let arg = &args[0];
            *output.borrow_mut() += &format!("[STDOUT] {}\n", arg);

            if let Some(report_output) = report_output.upgrade() {
                report_output(output.borrow().to_string())
            }
            Value::Bool(true)
        }),
    ));
    builtin_vars.add_native_fn("println", native_println, 1);
    builtin_vars.add_native_fn("is_nan", &builtin_functions::is_nan, 1);
    builtin_vars.add_native_fn("parse_number", &builtin_functions::parse_number, 1);
    builtin_vars.add_native_fn("clock", &native_clock, 0);
    builtin_vars.add_native_fn("str", &builtin_functions::str, 1);

    let dummy_source = "".into();
    let mut resolver = Resolver::new(&dummy_source);
    resolver.resolve_builtin_vars(&builtin_vars);
    let mut resolve_result = resolver.resolve_result();
    let accessible_symbols = resolver.accessible_symbols();

    let mut vm = Vm::new(&builtin_vars);
    let mut codegen = Codegen::new("<global>".to_string(), resolve_result, &source);
    codegen.codegen_builtin_vars(&builtin_vars);
    vm.interpret(codegen.into_inner_chunk()); // load built in functions into memory

    let mut parser = Parser::new(&source);
    let ast = parser.parse_program();

    let mut resolver =
        Resolver::new_with_existing_accessible_symbols(&source, accessible_symbols.clone());
    resolver.resolve_top_level(&ast);
    resolve_result = resolver.resolve_result();

    if source.has_no_errors() {
        let mut codegen = Codegen::new("<global>".to_string(), resolve_result, &source);

        codegen.codegen_function(&ast);

        let chunk = codegen.into_inner_chunk();
        let result = vm.interpret(chunk);

        if result != InterpretResult::Ok {
            report_errors(format!("{:?}", result));
        }

        let end = js_clock();
        *output.borrow_mut() +=
            &format!("[INFO] Execution finished in {:.3} seconds\n", end - start);
        if let Some(report_output) = report_output.upgrade() {
            report_output(output.borrow().to_string());
        }
    } else {
        let errors_string = format!("{}", source);
        report_errors(errors_string);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    ExecuteCode(String),
}

#[derive(Clone)]
pub struct Runner {
    link: AgentLink<Self>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RunResult {
    Stdout(String),
    Error(String),
}

impl Agent for Runner {
    type Reach = Job<Self>;
    type Message = ();
    type Input = Request;
    type Output = RunResult;

    fn create(link: AgentLink<Self>) -> Self {
        Self { link }
    }

    fn update(&mut self, _msg: Self::Message) {}

    fn handle_input(&mut self, msg: Self::Input, id: HandlerId) {
        match msg {
            Request::ExecuteCode(source) => {
                let report_output = Rc::new(enc!(
                    (self => runner, id) move |output: String| {
                        runner
                            .link
                            .respond(id, RunResult::Stdout(output.to_string()))
                    }
                ));
                let report_errors = Rc::new(|errors: String| {
                    self.link.respond(id, RunResult::Error(errors.to_string()))
                });

                run(Rc::new(source), report_output, report_errors);
            }
        }
    }
}
