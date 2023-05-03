use dipa::DiffPatch;
use leptos::*;
use leptos_server_signal::create_server_signal;

#[derive(Clone, Default, DiffPatch)]
pub struct Count {
    pub value: i32,
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Provide websocket connection
    leptos_server_signal::provide_websocket(cx, "ws://localhost:3000/ws").unwrap();

    // Create server signal
    let count = create_server_signal::<Count>(cx);

    view! { cx,
        <h1>"Count: " {move || count().value.to_string()}</h1>
    }
}
