use leptos::prelude::*;
use leptos_server_signal::create_server_signal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Count {
    pub value: i32,
}

/// Renders the full HTML document shell. The `<HydrationScripts>` element is
/// what injects the `<script>` tags that load the WASM bundle, without which
/// the client never hydrates (and the websocket is never opened).
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provide websocket connection
    leptos_server_signal::provide_websocket("ws://localhost:3000/ws").unwrap();

    // Create server signal
    let count = create_server_signal::<Count>("counter");

    view! { <h1>"Count: " {move || count.get().value.to_string()}</h1> }
}
