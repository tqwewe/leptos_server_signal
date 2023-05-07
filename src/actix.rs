use std::borrow::Cow;
use std::{fmt, ops};

use actix_ws::Session;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::ServerSignalUpdate;

/// A signal owned by the server which writes to the websocket when mutated.
#[derive(Clone)]
pub struct ServerSignal<T> {
    name: Cow<'static, str>,
    value: T,
    json_value: Value,
    session: Session,
}

impl<T> ServerSignal<T> {
    /// Creates a new [`ServerSignal`], initializing `T` to default.
    ///
    /// This function can fail if serilization of `T` fails.
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        session: Session,
    ) -> Result<Self, serde_json::Error>
    where
        T: Default + Serialize,
    {
        Ok(ServerSignal {
            name: name.into(),
            value: T::default(),
            json_value: serde_json::to_value(T::default())?,
            session,
        })
    }

    /// Modifies the signal in a closure, and sends the json diffs through the websocket connection after modifying.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let count = ServerSignal::new("counter", websocket).unwrap();
    /// count.with(|count| {
    ///     count.value += 1;
    /// }).await?;
    /// ```
    pub async fn with<'e, O>(&'e mut self, f: impl FnOnce(&mut T) -> O) -> Result<O, Error>
    where
        T: Clone + Serialize + 'static,
    {
        let output = f(&mut self.value);
        let new_json = serde_json::to_value(self.value.clone())?;
        let update =
            ServerSignalUpdate::new_from_json::<T>(self.name.clone(), &self.json_value, &new_json);
        let update_json = serde_json::to_string(&update)?;
        self.session.text(update_json).await?;
        self.json_value = new_json;
        Ok(output)
    }

    /// Consumes the [`ServerSignal`], returning the inner value.
    pub fn into_value(self) -> T {
        self.value
    }

    /// Consumes the [`ServerSignal`], returning the inner json value.
    pub fn into_json_value(self) -> Value {
        self.json_value
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

/// A server signal error.
#[derive(Debug, Error)]
pub enum Error {
    /// Serialization of the signal value failed.
    #[error(transparent)]
    SerializationFailed(#[from] serde_json::Error),
    /// The websocket was closed.
    #[error(transparent)]
    WebSocket(#[from] actix_ws::Closed),
}
