//! **Leptos Server Signals**
//!
//! Server signals are [leptos] [signals] kept in sync with the server through websockets.
//!
//! The signals are read-only on the client side, and can be written to by the server.
//! This is useful if you want real-time updates on the UI controlled by the server.
//!
//! Changes to a signal are sent through a websocket to the client as diffs,
//! containing only what has been changed, courtesy of the [dipa] crate.
//! The data is encoded using cbor encoding and sent via binary websocket messages.
//!
//! [leptos]: https://crates.io/crates/leptos
//! [signals]: https://docs.rs/leptos/latest/leptos/struct.Signal.html
//! [dipa]: https://crates.io/crates/dipa
//!
//! ## Feature flags
//!
//! - `ssr`: ssr is enabled when rendering the app on the server.
//! - `actix`: integration with the [Actix] web framework.
//! - `axum`: integration with the [Axum] web framework.
//!
//! [actix]: https://crates.io/crates/actix-web
//! [axum]: https://crates.io/crates/axum
//!
//! # Example
//!
//! **Cargo.toml**
//!
//! ```toml
//! [dependencies]
//! dipa = { version = "*", features = ["derive"] } # Used for diffing with serde
//! leptos_server_signal = "*"
//!
//! [features]
//! ssr = [
//!   "leptos_server_signal/ssr",
//!   "leptos_server_signal/axum", # or actix
//! ]
//! ```
//!
//! **Client**
//!
//! ```ignore
//! use dipa::DiffPatch;
//! use leptos::*;
//! use leptos_server_signal::create_server_signal;
//!
//! #[derive(Clone, Default, DiffPatch)]
//! pub struct Count {
//!     pub value: i32,
//! }
//!
//! #[component]
//! pub fn App(cx: Scope) -> impl IntoView {
//!     // Provide websocket connection
//!     leptos_server_signal::provide_websocket(cx, "ws://localhost:3000/ws").unwrap();
//!
//!     // Create server signal
//!     let count = create_server_signal::<Count>(cx);
//!
//!     view! { cx,
//!         <h1>"Count: " {move || count().value.to_string()}</h1>
//!     }
//! }
//! ```
//!
//! **Server (Axum)**
//!
//! ```ignore
//! #[cfg(feature = "ssr")]
//! pub async fn websocket(ws: WebSocketUpgrade) -> Response {
//!     ws.on_upgrade(handle_socket)
//! }
//!
//! #[cfg(feature = "ssr")]
//! async fn handle_socket(mut socket: WebSocket) {
//!     let mut count = ServerSignal::<Count>::new(websocket);
//!
//!     loop {
//!         tokio::time::sleep(Duration::from_millis(10)).await;
//!         if count.with(&mut socket, |count| count.value += 1).await.is_err() {
//!             break;
//!         }
//!     }
//! }
//! ```

use std::{any, borrow::Cow, io};

use dipa::{Diffable, Patchable};
use leptos::{create_signal, ReadSignal, Scope};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

cfg_if::cfg_if! {
    if #[cfg(all(feature = "actix", feature = "ssr"))] {
        mod actix;
        pub use crate::actix::*;
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(feature = "axum", feature = "ssr"))] {
        mod axum;
        pub use crate::axum::*;
    }
}

/// A server signal update containing the signal type name and diff delta.
///
/// This is whats sent over the websocket, and is used to patch the signal with the diff,
/// if the type name matches.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServerSignalUpdate {
    type_name: Cow<'static, str>,
    delta: Vec<u8>,
}

impl ServerSignalUpdate {
    /// Creates a new [`ServerSignalUpdate`] from an old and new instance of `T`.
    pub fn new<'s, 'e, T>(old: &'s T, new: &'e T) -> Self
    where
        T: Diffable<'s, 'e, T>,
        <T as Diffable<'s, 'e, T>>::Delta: Serialize,
    {
        let delta = old.create_delta_towards(new);
        let mut encoded = Vec::new();
        ciborium::ser::into_writer(&delta.delta, &mut encoded).unwrap();
        ServerSignalUpdate {
            type_name: any::type_name::<T>().into(),
            delta: encoded,
        }
    }

    /// Encodes the [`ServerSignalUpdate`] using cbor encoding.
    pub fn encode(&self) -> Result<Vec<u8>, ciborium::ser::Error<io::Error>> {
        let mut encoded = Vec::new();
        ciborium::ser::into_writer(self, &mut encoded)?;
        Ok(encoded)
    }
}

/// Provides a websocket url for server signals, if there is not already one provided.
/// This ensures that you can provide it at the highest possible level, without overwriting a websocket
/// that has already been provided (for example, by a server-rendering integration.)
///
/// Note, the server should have a route to handle this websocket.
///
/// # Example
///
/// ```ignore
/// #[component]
/// pub fn App(cx: Scope) -> impl IntoView {
///     // Provide websocket connection
///     leptos_server_signal::provide_websocket(cx, "ws://localhost:3000/ws").unwrap();
///     
///     // ...
/// }
/// ```
#[allow(unused_variables)]
pub fn provide_websocket(cx: Scope, url: &str) -> Result<(), JsValue> {
    provide_websocket_inner(cx, url)
}

/// Creates a signal which is controlled by the server.
///
/// This signal is initialized as T::default, is read-only on the client, and is updated through diffs
/// sent through a websocket connection.
///
/// # Example
///
/// ```
/// #[derive(Clone, Default, DiffPatch)]
/// pub struct Count {
///     pub value: i32,
/// }
///
/// #[component]
/// pub fn App(cx: Scope) -> impl IntoView {
///     // Create server signal
///     let count = create_server_signal::<Count>(cx);
///
///     view! { cx,
///         <h1>"Count: " {move || count().value.to_string()}</h1>
///     }
/// }
/// ```
pub fn create_server_signal<T>(cx: Scope) -> ReadSignal<T>
where
    T: Default
        + for<'s, 'e> Diffable<'s, 'e, T>
        + for<'s, 'e> Patchable<<T as Diffable<'s, 'e, T>>::DeltaOwned>,
    for<'s, 'e, 'de> <T as Diffable<'s, 'e, T>>::DeltaOwned: Deserialize<'de>,
{
    #[allow(unused_variables)]
    let (get, set) = create_signal(cx, T::default());

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            use web_sys::MessageEvent;
            use wasm_bindgen::{prelude::Closure, JsCast};
            use leptos::{use_context, create_effect, SignalUpdate};
            use js_sys::{Function, Uint8Array, ArrayBuffer};

            let ws = use_context::<ServerSignalWebSocket>(cx);

            match ws {
                Some(ServerSignalWebSocket(ws)) => {
                    create_effect(cx, move |_| {
                        let callback = Closure::wrap(Box::new(move |event: MessageEvent| {
                            let array_buffer = event.data().dyn_into::<ArrayBuffer>().unwrap();
                            let data = Uint8Array::new(&array_buffer).to_vec();
                            let update_signal: ServerSignalUpdate =
                                ciborium::de::from_reader(data.as_slice()).unwrap();
                            if update_signal.type_name == any::type_name::<T>() {
                                let delta =
                                    ciborium::de::from_reader(update_signal.delta.as_slice()).unwrap();
                                set.update(|value| {
                                    value.apply_patch(delta);
                                });
                            }
                        }) as Box<dyn FnMut(_)>);
                        let function: &Function = callback.as_ref().unchecked_ref();
                        ws.set_onmessage(Some(function));

                        // Keep the closure alive for the lifetime of the program
                        callback.forget();
                    });
                }
                None => {
                    leptos::error!(
                        r#"server signal was used without a websocket being provided.

Ensure you call `leptos_server_signal::provide_websocket(cx, "ws://localhost:3000/ws")` at the highest level in your app."#
                    );
                }
            }
        }
    }

    get
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use web_sys::{BinaryType, WebSocket};
        use leptos::{provide_context, use_context};

        #[derive(Clone, Debug, PartialEq, Eq)]
        struct ServerSignalWebSocket(WebSocket);

        #[inline]
        fn provide_websocket_inner(cx: Scope, url: &str) -> Result<(), JsValue> {
            if use_context::<ServerSignalWebSocket>(cx).is_none() {
                let ws = WebSocket::new(url)?;
                ws.set_binary_type(BinaryType::Arraybuffer);
                provide_context(cx, ServerSignalWebSocket(ws));
            }

            Ok(())
        }
    } else {
        #[inline]
        fn provide_websocket_inner(_cx: Scope, _url: &str) -> Result<(), JsValue> {
            Ok(())
        }
    }
}
