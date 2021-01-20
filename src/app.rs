use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use ella_parser::parser::Parser;
use ella_passes::resolve::Resolver;
use ella_value::BuiltinVars;
use ella_value::Value;
use ella_vm::codegen::Codegen;
use ella_vm::vm::{InterpretResult, Vm};

use enclose::enc;
use log::*;
use wasm_bindgen::prelude::*;
use yew::format::Nothing;
use yew::prelude::*;
use yew::services::fetch::{Request, Response};
use yew::services::{FetchService, TimeoutService};
use yew::utils::window;
use yew_functional::*;

static EXAMPLES: &[&str] = &["hello-world", "fibonacci", "speed-test"];

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

fn run(
    source: Rc<String>,
    report_output: Rc<impl Fn(&str) + 'static>,
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
                report_output(output.borrow().as_str())
            }
            Value::Bool(true)
        }),
    ));
    builtin_vars.add_native_fn("println", native_println, 1);
    builtin_vars.add_native_fn("clock", &native_clock, 0);

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
            report_output(output.borrow().as_str())
        }
    } else {
        let errors_string = format!("{}", source);
        report_errors(errors_string);
    }
}

#[function_component(App)]
pub fn app() -> Html {
    info!("rendered");

    let (source, set_source) = use_state(|| "".to_string());
    let (output, set_output) = use_state(|| "".to_string());
    let (is_loading, set_is_loading) = use_state(|| false);
    let (is_error, set_is_error) = use_state(|| false);
    let timeout_handle = use_ref(|| None);
    let fetch_example_task_handle = use_ref(|| None);
    let (examples_dropdown_open, set_examples_dropdown_open) = use_state(|| false);

    let report_output = Rc::new(enc!((set_output) move |new_output: &str| {
        set_output(new_output.to_string());
    }));

    let report_errors = Rc::new(
        enc!((set_output, set_is_error) move |errors_string: String| {
            set_output(errors_string);
            set_is_error(true);
        }),
    );

    let handle_run = Callback::from(
        enc!((source, report_output, report_errors, set_output, set_is_loading, set_is_error, timeout_handle) move |_| {
            set_output("".to_string());
            set_is_loading(true);
            set_is_error(false);

            let handle = TimeoutService::spawn(Duration::from_secs(0), Callback::from(enc!((source, report_output, report_errors, set_is_loading) move |_| {
                run(Rc::clone(&source), Rc::clone(&report_output), Rc::clone(&report_errors));
                set_is_loading(false);
            })));
            *timeout_handle.borrow_mut() = Some(handle);
        }),
    );

    let close_dropdown = Callback::from(enc!((set_examples_dropdown_open) move |_| {
        set_examples_dropdown_open(false);
    }));

    let toggle_dropdown = Callback::from(
        enc!((examples_dropdown_open, set_examples_dropdown_open) move |event: MouseEvent| {
            event.stop_immediate_propagation();
            set_examples_dropdown_open(!*examples_dropdown_open);
        }),
    );

    let load_example = Rc::new(Callback::from(enc!(
        (set_source, fetch_example_task_handle) move |name| {
            info!("loading example {}", name);
            let location = window().location();
            let url = format!("{}{}examples/{}.ella", location.origin().unwrap(), location.pathname().unwrap(), name);
            let req = Request::get(url)
                .body(Nothing)
                .unwrap();

            let callback = Callback::from(enc!((set_source) move |response: Response<anyhow::Result<String>>| {
                if let (meta, Ok(response)) = response.into_parts() {
                    if meta.status.is_success() {
                        set_source(response);
                    } else {
                        set_source("Error, could not fetch example.".to_string());
                    }
                }
            }));
            let task = FetchService::fetch(req, callback);
            *fetch_example_task_handle.borrow_mut() = Some(task);
        }
    )));

    html! {
        <main class="m-3" onclick=close_dropdown>
            <div class="columns">
                <div class="column header">{ "Ellalang Playground" }</div>

                <div class="column">
                    <button
                        class=format!("button is-primary {}", if *is_loading { "is-loading" } else { "" })
                        disabled=*is_loading
                        onclick=handle_run
                    >{ "Run" }</button>
                </div>

                <div class="column">
                    <div class=format!("dropdown {}", if *examples_dropdown_open { "is-active" } else { "" })>
                        <button class="button dropdown-trigger" onclick=toggle_dropdown>{ "Example scripts" }</button>
                        <div class="dropdown-menu" id="dropdown-menu" role="menu">
                            <div class="dropdown-content">
                                { for EXAMPLES.iter().map(|name| html! {
                                    <a
                                        href="#"
                                        class="dropdown-item"
                                        onclick=Callback::from(enc!((load_example) move |_| load_example.emit(name)))
                                    >{ name }</a>
                                })}
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <div class="columns input-area">
                <div class="column">
                    <div class="control">
                        <textarea
                            class="textarea"
                            placeholder="Source code here..."
                            spellcheck=false
                            value=source
                            oninput=Callback::from(enc!((set_source) move |ev: InputData| set_source(ev.value)))
                        />
                    </div>
                </div>

                <div class="column">
                    <div class=format!("control {}", if *is_loading { "is-loading" } else { "" })>
                        <textarea
                            class=format!("textarea column {}", if *is_error { "is-danger" } else { "" })
                            readonly=true
                            spellcheck=false
                            value=output
                        />
                    </div>
                </div>
            </div>
        </main>
    }
}
