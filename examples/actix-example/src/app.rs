use leptos::*;
use leptos_server_signal::create_server_signal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Count {
    pub value: i32,
}

#[component]
pub fn App() -> impl IntoView {
    // Provide websocket connection
    leptos_server_signal::provide_websocket("ws://localhost:3000/ws").unwrap();

    // Create server signal
    let count = create_server_signal::<Count>("counter");

    view! {
        <h1>"Count: " {move || count().value.to_string()}</h1>
    }
}
