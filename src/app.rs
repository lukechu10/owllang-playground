use std::rc::Rc;
use std::time::Duration;

use enclose::enc;
use log::*;
use runner::{RunResult, Runner};
use yew::agent::Bridged;
use yew::format::Nothing;
use yew::prelude::*;
use yew::utils::window;
use yew_functional::*;
use yew_services::fetch::{Request, Response};
use yew_services::{FetchService, TimeoutService};

use crate::runner;

static EXAMPLES: &[&str] = &["hello-world", "factorial", "fibonacci", "speed-test"];

#[function_component(App)]
pub fn app() -> Html {
    info!("rendered");

    let source = use_state(|| "".to_string());
    let output = use_state(|| "".to_string());
    let is_loading = use_state(|| false);
    let is_error = use_state(|| false);
    let timeout_handle = use_ref(|| None);
    let fetch_example_task_handle = use_ref(|| None);
    let examples_dropdown_open = use_state(|| false);

    let report_output = Rc::new(enc!((output) move |new_output: String| {
        output.set(new_output);
    }));

    let report_errors = Rc::new(enc!((output, is_error) move |errors_string: String| {
        output.set(errors_string);
        is_error.set(true);
    }));

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
        enc!((source, output, is_loading, is_error, timeout_handle) move |_| {
            output.set("".to_string());
            is_loading.set(true);
            is_error.set(false);

            let handle = TimeoutService::spawn(
                Duration::from_secs(0),
                Callback::from(enc!((source, runner_handle, is_loading) move |_| {
                    runner_handle.borrow_mut().send(runner::Request::ExecuteCode(source.to_string()));
                    is_loading.set(false);
                })),
            );
            *timeout_handle.borrow_mut() = Some(handle);
        }),
    );

    let close_dropdown = Callback::from(enc!((examples_dropdown_open) move |_| {
        examples_dropdown_open.set(false);
    }));

    let toggle_dropdown = Callback::from(enc!((examples_dropdown_open) move |event: MouseEvent| {
        event.stop_immediate_propagation();
        examples_dropdown_open.set(!*examples_dropdown_open);
    }));

    let load_example = Rc::new(Callback::from(enc!(
        (source, fetch_example_task_handle) move |name| {
            info!("loading example {}", name);
            let location = window().location();
            let url = format!("{}{}examples/{}.hoot", location.origin().unwrap(), location.pathname().unwrap(), name);
            let req = Request::get(url)
                .body(Nothing)
                .unwrap();

            let callback = Callback::from(enc!((source) move |response: Response<anyhow::Result<String>>| {
                if let (meta, Ok(response)) = response.into_parts() {
                    if meta.status.is_success() {
                        source.set(response);
                    } else {
                        source.set("Error, could not fetch example.".to_string());
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
                <div class="column header">{ "owllang Playground" }</div>

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
                            value=*source
                            oninput=Callback::from(enc!((source) move |ev: InputData| source.set(ev.value)))
                        />
                    </div>
                </div>

                <div class="column">
                    <div class=format!("control {}", if *is_loading { "is-loading" } else { "" })>
                        <textarea
                            class=format!("textarea column {}", if *is_error { "is-danger" } else { "" })
                            readonly=true
                            spellcheck=false
                            value=*output
                        />
                    </div>
                </div>
            </div>
        </main>
    }
}
