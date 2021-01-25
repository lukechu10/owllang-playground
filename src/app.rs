use std::rc::Rc;
use std::time::Duration;

use enclose::enc;
use log::*;
use runner::{RunResult, Runner};
use yew::agent::Bridged;
use yew::format::Nothing;
use yew::prelude::*;
use yew::services::fetch::{Request, Response};
use yew::services::{FetchService, TimeoutService};
use yew::utils::window;
use yew_functional::*;

use crate::runner;

static EXAMPLES: &[&str] = &["hello-world", "factorial", "fibonacci", "speed-test"];

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

    let report_output = Rc::new(enc!((set_output) move |new_output: String| {
        set_output(new_output);
    }));

    let report_errors = Rc::new(
        enc!((set_output, set_is_error) move |errors_string: String| {
            set_output(errors_string);
            set_is_error(true);
        }),
    );

    let callback = Callback::from(
        enc!((report_output, report_errors) move |result: RunResult| {
            match result {
                RunResult::Stdout(stdout) => report_output(stdout),
                RunResult::Error(err) => report_errors(err),
            }
        }),
    );
    let runner_handle = use_ref(|| Runner::bridge(callback));

    let handle_run = Callback::from(
        enc!((source, set_output, set_is_loading, set_is_error, timeout_handle) move |_| {
            set_output("".to_string());
            set_is_loading(true);
            set_is_error(false);

            let handle = TimeoutService::spawn(Duration::from_secs(0), Callback::from(enc!((source, runner_handle, set_is_loading) move |_| {
                runner_handle.borrow_mut().send(runner::Request::ExecuteCode(source.to_string()));
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
