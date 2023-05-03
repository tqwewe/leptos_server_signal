**Leptos Server Signals**

Server signals are [leptos] [signals] kept in sync with the server through websockets.

The signals are read-only on the client side, and can be written to by the server.
This is useful if you want real-time updates on the UI controlled by the server.

Changes to a signal are sent through a websocket to the client as diffs,
containing only what has been changed, courtesy of the [dipa] crate.
The data is encoded using cbor encoding and sent via binary websocket messages.

[leptos]: https://crates.io/crates/leptos
[signals]: https://docs.rs/leptos/latest/leptos/struct.Signal.html
[dipa]: https://crates.io/crates/dipa

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
dipa = { version = "*", features = ["derive"] } # Used for diffing with serde
leptos_server_signal = "*"

[features]
ssr = [
  "leptos_server_signal/ssr",
  "leptos_server_signal/axum", # or actix
]
```

**Client**

```
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
```

**Server (Axum)**

```ignore
#[cfg(feature = "ssr")]
pub async fn websocket(ws: WebSocketUpgrade) -> axum::response::Response {
    ws.on_upgrade(handle_socket)
}

#[cfg(feature = "ssr")]
async fn handle_socket(socket: WebSocket) {
    let websocket = Arc::new(Mutex::new(socket));
    let mut count = ServerSignal::<Count>::new(websocket);

    loop {
        tokio::time::sleep(Duration::from_millis(10)).await;
        if count.with(|count| count.value += 1).await.is_err() {
            break;
        }
    }
}
```
