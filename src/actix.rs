use std::{fmt, ops};

use actix_ws::Session;
use dipa::Diffable;
use serde::Serialize;

use crate::ServerSignalUpdate;

/// A signal owned by the server which writes to the websocket when mutated.
///
/// A `ServerSignal` contains the value `T`, and a [`Session`] instance.
#[derive(Clone)]
pub struct ServerSignal<T> {
    value: T,
    session: Session,
}

impl<T> ServerSignal<T> {
    /// Creates a new [`ServerSignal`] from a [`Session`].
    pub fn new(session: Session) -> Self
    where
        T: Default,
    {
        ServerSignal {
            value: T::default(),
            session,
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
    pub async fn with<'e, O>(
        &'e mut self,
        f: impl FnOnce(&mut T) -> O,
    ) -> Result<O, actix_ws::Closed>
    where
        T: Clone + for<'s> Diffable<'s, 'e, T>,
        for<'s> <T as Diffable<'s, 'e, T>>::Delta: Serialize,
    {
        let old = self.value.clone();
        let output = f(&mut self.value);
        let update = ServerSignalUpdate::new(&old, &self.value);
        self.session.binary(update.encode().unwrap()).await?;
        Ok(output)
    }

    /// Consumes the [`ServerSignal`], returning the inner value.
    pub fn into_value(self) -> T {
        self.value
    }
}

impl<T> fmt::Debug for ServerSignal<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ServerSignal({:?})", self.value)
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
