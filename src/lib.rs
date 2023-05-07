#![doc = include_str!("../README.md")]

use std::borrow::Cow;

use json_patch::Patch;
use leptos::{create_signal, ReadSignal, Scope};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

/// A server signal update containing the signal type name and json patch.
///
/// This is whats sent over the websocket, and is used to patch the signal if the type name matches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerSignalUpdate {
    name: Cow<'static, str>,
    patch: Patch,
}

impl ServerSignalUpdate {
    /// Creates a new [`ServerSignalUpdate`] from an old and new instance of `T`.
    pub fn new<'s, 'e, T>(
        name: impl Into<Cow<'static, str>>,
        old: &'s T,
        new: &'e T,
    ) -> Result<Self, serde_json::Error>
    where
        T: Serialize,
    {
        let left = serde_json::to_value(old)?;
        let right = serde_json::to_value(new)?;
        let patch = json_patch::diff(&left, &right);
        Ok(ServerSignalUpdate {
            name: name.into(),
            patch,
        })
    }

    /// Creates a new [`ServerSignalUpdate`] from two json values.
    pub fn new_from_json<'s, 'e, T>(
        name: impl Into<Cow<'static, str>>,
        old: &Value,
        new: &Value,
    ) -> Self {
        let patch = json_patch::diff(old, new);
        ServerSignalUpdate {
            name: name.into(),
            patch,
        }
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
/// This signal is initialized as T::default, is read-only on the client, and is updated through json patches
/// sent through a websocket connection.
///
/// # Example
///
/// ```
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// pub struct Count {
///     pub value: i32,
/// }
///
/// #[component]
/// pub fn App(cx: Scope) -> impl IntoView {
///     // Create server signal
///     let count = create_server_signal::<Count>(cx, "counter");
///
///     view! { cx,
///         <h1>"Count: " {move || count().value.to_string()}</h1>
///     }
/// }
/// ```
#[allow(unused_variables)]
pub fn create_server_signal<T>(cx: Scope, name: impl Into<Cow<'static, str>>) -> ReadSignal<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de>,
{
    let name: Cow<'static, str> = name.into();
    let (get, set) = create_signal(cx, T::default());

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            use web_sys::MessageEvent;
            use wasm_bindgen::{prelude::Closure, JsCast};
            use leptos::{use_context, create_effect, SignalUpdate};
            use js_sys::{Function, JsString};

            let (json_get, json_set) = create_signal(cx, serde_json::to_value(T::default()).unwrap());
            let ws = use_context::<ServerSignalWebSocket>(cx);

            match ws {
                Some(ServerSignalWebSocket(ws)) => {
                    create_effect(cx, move |_| {
                        let name = name.clone();
                        let callback = Closure::wrap(Box::new(move |event: MessageEvent| {
                            let ws_string = event.data().dyn_into::<JsString>().unwrap().as_string().unwrap();
                            if let Ok(update_signal) = serde_json::from_str::<ServerSignalUpdate>(&ws_string) {
                                if update_signal.name == name {
                                    json_set.update(|doc| {
                                        json_patch::patch(doc, &update_signal.patch).unwrap();
                                    });
                                    let new_value = serde_json::from_value(json_get()).unwrap();
                                    set(new_value);
                                }
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
        use web_sys::WebSocket;
        use leptos::{provide_context, use_context};

        #[derive(Clone, Debug, PartialEq, Eq)]
        struct ServerSignalWebSocket(WebSocket);

        #[inline]
        fn provide_websocket_inner(cx: Scope, url: &str) -> Result<(), JsValue> {
            if use_context::<ServerSignalWebSocket>(cx).is_none() {
                let ws = WebSocket::new(url)?;
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
