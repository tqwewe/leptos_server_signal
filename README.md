**Leptos Server Signals**

Server signals are [leptos] [signals] kept in sync with the server through websockets.

The signals are read-only on the client side, and can be written to by the server.
This is useful if you want real-time updates on the UI controlled by the server.

Changes to a signal are sent through a websocket to the client as [json patches].

[leptos]: https://crates.io/crates/leptos
[signals]: https://docs.rs/leptos/latest/leptos/struct.Signal.html
[json patches]: https://docs.rs/json-patch/latest/json_patch/struct.Patch.html

## Feature flags

- `ssr`: ssr is enabled when rendering the app on the server.
- `actix`: integration with the [Actix] web framework.
- `axum`: integration with the [Axum] web framework.

[actix]: https://crates.io/crates/actix-web
[axum]: https://crates.io/crates/axum

# Example

**Cargo.toml**

```toml
[dependencies]
leptos_server_signal = "*"
serde = { version = "*", features = ["derive"] }

[features]
ssr = [
  "leptos_server_signal/ssr",
  "leptos_server_signal/axum", # or actix
]
```

**Client**

```rust
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
```

> If on stable, use `count.get().value` instead of `count().value`.

**Server (Axum)**

```rust
#[cfg(feature = "ssr")]
pub async fn websocket(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

#[cfg(feature = "ssr")]
async fn handle_socket(mut socket: WebSocket) {
    let mut count = ServerSignal::<Count>::new("counter").unwrap();

    loop {
        tokio::time::sleep(Duration::from_millis(10)).await;
        let result = count.with(&mut socket, |count| count.value += 1).await;
        if result.is_err() {
            break;
        }
    }
}
```
