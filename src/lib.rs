#![doc = include_str!("../README.md")]

use std::borrow::Cow;

use json_patch::Patch;
use leptos::{create_signal, ReadSignal};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::JsValue;
use web_sys::WebSocket;

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
    pub fn new<T>(
        name: impl Into<Cow<'static, str>>,
        old: &T,
        new: &T,
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
    pub fn new_from_json<T>(name: impl Into<Cow<'static, str>>, old: &Value, new: &Value) -> Self {
        let patch = json_patch::diff(old, new);
        ServerSignalUpdate {
            name: name.into(),
            patch,
        }
    }
}

/// Provides a websocket url for server signals, if there is not already one provided.
///
/// During SSR, this function is a no-op and returns `Ok(None)`.
/// During CSR, if this function returns `Ok`, then the `Option` will always be `Some`.
///
/// Note, the server should have a route to handle this websocket.
///
/// # Example
///
/// ```ignore
/// #[component]
/// pub fn App() -> impl IntoView {
///     // Provide websocket connection
///     leptos_server_signal::provide_websocket("ws://localhost:3000/ws").unwrap();
///
///     // ...
/// }
/// ```
#[allow(unused_variables)]
pub fn provide_websocket(url: &str) -> Result<Option<WebSocket>, JsValue> {
    provide_websocket_inner(url)
}

/// Provides a websocket url for server signals, if there is not already one provided.
/// In case of a connection lost, the websocket will be reconnected after the specified 
/// timeout.
///
/// During SSR, this function is a no-op and returns `Ok(None)`.
/// During CSR, if this function returns `Ok`, then the `Option` will always be `Some`.
///
/// Note, the server should have a route to handle this websocket.
///
/// # Example
///
/// ```ignore
/// #[component]
/// pub fn App() -> impl IntoView {
///     // Provide websocket connection
///     leptos_server_signal::provide_websocket_with_retry(
///         "ws://localhost:3000/ws",
///         5000, // retry to connect after 5 seconds
///     ).unwrap();
///
///     // ...
/// }
/// ```
pub fn provide_websocket_with_retry(
    url: &str,
    timeout_in_ms: i32
) -> Result<Option<WebSocket>, JsValue> {
    let ws = provide_websocket_inner(url);
    if let Ok(Some(ref ws)) = ws {
        add_retry_timeout(&ws, timeout_in_ms);
    }
    ws
}

/// Creates a signal which is controlled by the server.
///
/// This signal is initialized as T::default, is read-only on the client, and is updated through json patches
/// sent through a websocket connection.
///
/// # Example
///
/// ```
/// # use leptos::{component, view, IntoView, SignalGet};
/// # use serde::{Deserialize, Serialize};
/// # use leptos_server_signal::create_server_signal;
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// pub struct Count {
///     pub value: i32,
/// }
///
/// #[component]
/// pub fn App() -> impl IntoView {
///     // Create server signal
///     let count = create_server_signal::<Count>("counter");
///
///     view! {
///         <h1>"Count: " {move || count.get().value.to_string()}</h1>
///     }
/// }
/// ```
#[allow(unused_variables)]
pub fn create_server_signal<T>(name: impl Into<Cow<'static, str>>) -> ReadSignal<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de>,
{
    let name: Cow<'static, str> = name.into();
    let (get, set) = create_signal(T::default());

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            use leptos::{use_context, create_effect, create_rw_signal, SignalGet, SignalSet};

            let signal = create_rw_signal(serde_json::to_value(T::default()).unwrap());
            if let Some(ServerSignalWebSocket { state_signals, .. }) = use_context::<ServerSignalWebSocket>() {
                let name: Cow<'static, str> = name.into();
                state_signals.borrow_mut().insert(name.clone(), signal);

                // Note: The leptos docs advise against doing this. It seems to work
                // well in testing, and the primary caveats are around unnecessary
                // updates firing, but our state synchronization already prevents
                // that on the server side
                create_effect(move |_| {
                    let name = name.clone();
                    let new_value = serde_json::from_value(signal.get()).unwrap();
                    set.set(new_value);
                });

            } else {
                leptos::logging::error!(
                    r#"server signal was used without a websocket being provided.

Ensure you call `leptos_server_signal::provide_websocket("ws://localhost:3000/ws")` at the highest level in your app."#
                );
            }

        }
    }

    get
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;

        use leptos::{provide_context, RwSignal};

        /// The websocket connection wrapper provided as a context in Leptos.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct ServerSignalWebSocket {
            ws: WebSocket,
            // References to these are kept by the closure for the callback
            // onmessage callback on the websocket
            state_signals: Rc<RefCell<HashMap<Cow<'static, str>, RwSignal<serde_json::Value>>>>,
            // When the websocket is first established, the leptos may not have
            // completed the traversal that sets up all of the state signals.
            // Without that, we don't have a base state to apply the patches to,
            // and therefore we must keep a record of the patches to apply after
            // the state has been set up.
            delayed_updates: Rc<RefCell<HashMap<Cow<'static, str>, Vec<Patch>>>>,
        }

        impl ServerSignalWebSocket {
            /// Returns the inner websocket.
            pub fn ws(&self) -> WebSocket {
                self.ws.clone()
            }
        }

        #[inline]
        fn provide_websocket_inner(url: &str) -> Result<Option<WebSocket>, JsValue> {
            use web_sys::MessageEvent;
            use wasm_bindgen::{prelude::Closure, JsCast};
            use leptos::{use_context, SignalUpdate};
            use js_sys::{Function, JsString};

            if use_context::<ServerSignalWebSocket>().is_none() {
                let ws = WebSocket::new(url)?;
                provide_context(ServerSignalWebSocket { ws, state_signals: Rc::default(), delayed_updates: Rc::default() });
            }

            let ws = use_context::<ServerSignalWebSocket>().unwrap();

            let handlers = ws.state_signals.clone();
            let delayed_updates = ws.delayed_updates.clone();

            let callback = Closure::wrap(Box::new(move |event: MessageEvent| {
                let ws_string = event.data().dyn_into::<JsString>().unwrap().as_string().unwrap();
                if let Ok(update_signal) = serde_json::from_str::<ServerSignalUpdate>(&ws_string) {
                    let handler_map = (*handlers).borrow();
                    let name = &update_signal.name;
                    let mut delayed_map = (*delayed_updates).borrow_mut();
                    if let Some(signal) = handler_map.get(name) {
                        if let Some(delayed_patches) = delayed_map.remove(name) {
                            signal.update(|doc| {
                                for patch in delayed_patches {
                                    json_patch::patch(doc, &patch).unwrap();
                                }
                            });
                        }
                        signal.update(|doc| {
                            json_patch::patch(doc, &update_signal.patch).unwrap();
                        });
                    } else {
                        leptos::logging::warn!("No local state for update to {}. Queuing patch.", name);
                        delayed_map.entry(name.clone()).or_default().push(update_signal.patch.clone());
                    }
                }
            }) as Box<dyn FnMut(_)>);
            let function: &Function = callback.as_ref().unchecked_ref();
            ws.ws.set_onmessage(Some(function));

            // Keep the closure alive for the lifetime of the program
            callback.forget();

            Ok(Some(ws.ws()))
        }

        #[inline]
        fn add_retry_timeout(ws: &WebSocket, timeout_in_ms: i32) {
            use web_sys::{MessageEvent, window};
            use wasm_bindgen::prelude::{Closure, JsCast};
            use leptos::use_context;
            use js_sys::Function;

            let mut server_signal_ws = use_context::<ServerSignalWebSocket>().unwrap();

            let on_timeout_callback = Closure::wrap(Box::new(move |_: MessageEvent| {
                leptos::logging::log!("Try to reconnect signal web-socket.");                
                let new_ws = WebSocket::new(server_signal_ws.ws.url().as_str()).unwrap();
                new_ws.set_onmessage(server_signal_ws.ws.onmessage().as_ref());
                new_ws.set_onclose(server_signal_ws.ws.onclose().as_ref());
                new_ws.set_onerror(server_signal_ws.ws.onerror().as_ref());
                server_signal_ws.ws = new_ws;
            }) as Box<dyn FnMut(_)>);

            let on_error_callback = Closure::wrap(Box::new(move |_: MessageEvent| {
                let on_timeout_function: &Function = on_timeout_callback.as_ref().unchecked_ref();
                leptos::logging::log!(
                    "Connection lost to signal web-socket. Try to reconnect in {} milliseconds.",
                    timeout_in_ms
                );
                let _ = window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(
                    on_timeout_function,
                    timeout_in_ms
                );
            }) as Box<dyn FnMut(_)>);
            let on_error_function: &Function = on_error_callback.as_ref().unchecked_ref();
            ws.set_onerror(Some(on_error_function));
            on_error_callback.forget();
        }
    } else {
        #[inline]
        fn provide_websocket_inner(_url: &str) -> Result<Option<WebSocket>, JsValue> {
            Ok(None)
        }

        #[inline]
        fn add_retry_timeout(_ws: &WebSocket, _timeout_in_ms: i32) {}        
    }
}
