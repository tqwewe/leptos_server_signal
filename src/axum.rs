use std::ops;

use axum::extract::ws::Message;
use dipa::Diffable;
use futures::sink::{Sink, SinkExt};
use serde::Serialize;

use crate::ServerSignalUpdate;

/// A signal owned by the server which writes to the when mutated.
#[derive(Clone, Debug)]
pub struct ServerSignal<T>(T);

impl<T> ServerSignal<T> {
    /// Creates a new [`ServerSignal`], initializing T to default.
    pub fn new() -> Self
    where
        T: Default,
    {
        ServerSignal(T::default())
    }

    /// Modifies the signal in a closure, and sends the diff through the websocket connection after modifying.
    ///
    /// The same websocket connection should be used for a given client, otherwise the signal could become out of sync.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let count = ServerSignal::new();
    /// count.with(&mut websocket, |count| {
    ///     count.value += 1;
    /// }).await?;
    /// ```
    pub async fn with<'e, O, S>(
        &'e mut self,
        sink: &mut S,
        f: impl FnOnce(&mut T) -> O,
    ) -> Result<O, axum::Error>
    where
        T: Clone + for<'s> Diffable<'s, 'e, T>,
        for<'s> <T as Diffable<'s, 'e, T>>::Delta: Serialize,
        S: Sink<Message> + Unpin,
        axum::Error: From<<S as Sink<Message>>::Error>,
    {
        let old = self.0.clone();
        let output = f(&mut self.0);
        let update = ServerSignalUpdate::new(&old, &self.0);
        sink.send(Message::Binary(update.encode().unwrap())).await?;
        Ok(output)
    }

    /// Consumes the [`ServerSignal`], returning the inner value.
    pub fn into_value(self) -> T {
        self.0
    }
}

impl<T> ops::Deref for ServerSignal<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> AsRef<T> for ServerSignal<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}
