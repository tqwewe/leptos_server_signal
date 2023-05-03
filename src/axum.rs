use std::{ops, sync::Arc};

use axum::extract::ws::{Message, WebSocket};
use dipa::Diffable;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::ServerSignalUpdate;

/// A signal owned by the server which writes to the websocket when mutated.
///
/// A `ServerSignal` contains the value `T`, and a [`WebSocket`] instance.
#[derive(Clone, Debug)]
pub struct ServerSignal<T> {
    value: T,
    websocket: Arc<Mutex<WebSocket>>,
}

impl<T> ServerSignal<T> {
    /// Creates a new [`ServerSignal`] from a shared [`WebSocket`].
    pub fn new(websocket: Arc<Mutex<WebSocket>>) -> Self
    where
        T: Default,
    {
        ServerSignal {
            value: T::default(),
            websocket,
        }
    }

    /// Modifies the signal in a closure, and sends the diff through the websocket connection after modifying.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let count = ServerSignal::new(websocket);
    /// count.with(|count| {
    ///     count.value += 1;
    /// }).await?;
    /// ```
    pub async fn with<'e, O>(&'e mut self, f: impl FnOnce(&mut T) -> O) -> Result<O, axum::Error>
    where
        T: Clone + for<'s> Diffable<'s, 'e, T>,
        for<'s> <T as Diffable<'s, 'e, T>>::Delta: Serialize,
    {
        let old = self.value.clone();
        let output = f(&mut self.value);
        let update = ServerSignalUpdate::new(&old, &self.value);
        self.websocket
            .lock()
            .await
            .send(Message::Binary(update.encode().unwrap()))
            .await?;
        Ok(output)
    }

    /// Consumes the [`ServerSignal`], returning the inner value.
    pub fn into_value(self) -> T {
        self.value
    }
}

impl<T> ops::Deref for ServerSignal<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> AsRef<T> for ServerSignal<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}
