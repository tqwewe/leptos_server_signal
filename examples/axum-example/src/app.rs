use leptos::*;
use leptos_server_signal::create_server_signal;
use serde::{Deserialize, Serialize};
use leptos::prelude::Get;
use leptos::prelude::ElementChild;
use leptos::prelude::*;
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Count {
    pub value: i32,
}

#[component]
pub fn App() -> impl IntoView {
    // Provide websocket connection
    //leptos_server_signal::provide_websocket("ws://localhost:3000/ws").unwrap();

    use leptos::logging::log;
    
    log!("Hello from Leptos WASM11!");
    // Create server signal
    let count = create_server_signal::<Count>("counter");
    let (get_count, set_count) = signal(3);

    view! { 
        <h1>"Count: " {move || count.get().value.to_string()}</h1> 
        <button
            on:click=move |_| set_count.set(3)
        >
            "Click me: "
            {get_count}
        </button>
    }
}

