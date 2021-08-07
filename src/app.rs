use std::rc::Rc;

use enclose::enc;
use gloo::timers::callback::Timeout;
use log::*;
use reqwasm::http::Request;
use runner::{RunResult, Runner};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_agent::Bridged;

use crate::runner;

static EXAMPLES: &[&str] = &["hello-world", "factorial", "fibonacci", "speed-test"];

#[function_component(Header)]
fn header() -> Html {
    html! {
        <div class="column header">{ "owllang Playground" }</div>
    }
}

#[function_component(App)]
pub fn app() -> Html {
    info!("rendered");

    let source = use_state(|| "".to_string());
    let output = use_state(|| "".to_string());
    let is_loading = use_state(|| false);
    let is_error = use_state(|| false);
    let timeout_handle = use_ref(|| None);
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

            let handle = Timeout::new(
                0,
                enc!((source, runner_handle, is_loading) move || {
                    runner_handle.borrow_mut().send(runner::Request::ExecuteCode(source.to_string()));
                    is_loading.set(false);
                }),
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
        (source) move |name| {
            info!("loading example {}", name);
            spawn_local(enc!((source) async move {
                let res = Request::get(&format!("examples/{}.hoot", name)).send().await.unwrap();
                if res.status() == 200 {
                    source.set(res.text().await.unwrap());
                } else {
                    source.set("Error, could not fetch example.".to_string());
                }
            }));
        }
    )));

    html! {
        <main class="m-3" onclick={close_dropdown}>
            <div class="columns">
                <Header />
                <div class="column">
                    <button
                        class={format!("button is-primary {}", if *is_loading { "is-loading" } else { "" })}
                        disabled={*is_loading}
                        onclick={handle_run}
                    >{ "Run" }</button>
                </div>

                <div class="column">
                    <div class={format!("dropdown {}", if *examples_dropdown_open { "is-active" } else { "" })}>
                        <button class="button dropdown-trigger" onclick={toggle_dropdown}>{ "Example scripts" }</button>
                        <div class="dropdown-menu" id="dropdown-menu" role="menu">
                            <div class="dropdown-content">
                                { for EXAMPLES.iter().map(|name| html! {
                                    <a
                                        href="#"
                                        class="dropdown-item"
                                        onclick={Callback::from(enc!((load_example) move |_| load_example.emit(name)))}
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
                            spellcheck="false"
                            value={(*source).clone()}
                            oninput={Callback::from(enc!((source) move |ev: InputData| source.set(ev.value)))}
                        />
                    </div>
                </div>

                <div class="column">
                    <div class={format!("control {}", if *is_loading { "is-loading" } else { "" })}>
                        <textarea
                            class={format!("textarea column {}", if *is_error { "is-danger" } else { "" })}
                            readonly=true
                            spellcheck="false"
                            value={(*output).clone()}
                        />
                    </div>
                </div>
            </div>
        </main>
    }
}
