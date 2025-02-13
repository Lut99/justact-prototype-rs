//  IO.rs
//    by Lut99
//
//  Created:
//    15 Jan 2025, 15:57:55
//  Last edited:
//    31 Jan 2025, 18:13:08
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines some wrapping layers for properly generating a system trace.
//!   
//!   You should observe an error relating to Dan trying to send Amy's message without having
//!   received it.
//

use std::borrow::Cow;
use std::error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::sync::{Arc, Mutex, OnceLock};

use crate::auditing::{Event, EventControl};
use crate::sets::{Agreements, MapAsync, MapAsyncView, Times};
use crate::wire::{Action, Agreement, Message};

mod justact {
    pub use ::justact::auxillary::Identifiable;
    pub use ::justact::collections::Recipient;
    pub use ::justact::collections::map::{Map, MapAsync, MapSync};
    pub use ::justact::collections::set::{Set, SetSync};
    pub use ::justact::times::{Times, TimesSync};
}


/***** GLOBALS *****/
/// Defines \*some\* [`EventHandler`] that will handle trace callbacks.
pub(crate) static EVENT_HANDLER: OnceLock<Mutex<Box<dyn EventHandler>>> = OnceLock::new();





/***** ERRORS *****/
/// Defines errors that are emitted by the [`EventHandler`].
#[derive(Debug)]
pub enum Error<E> {
    /// It's the error of the inner one.
    Inner(E),
    /// Failed to handle a trace.
    EventHandle { err: Box<dyn 'static + Send + error::Error> },
}
impl<E: Display> Display for Error<E> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        match self {
            Self::Inner(err) => err.fmt(f),
            Self::EventHandle { .. } => write!(f, "Failed to handle trace with registered handler"),
        }
    }
}
impl<E: error::Error> error::Error for Error<E> {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Inner(err) => err.source(),
            Self::EventHandle { err } => Some(&**err),
        }
    }
}





/***** INTERFACES *****/
/// Defines a general trace handler implementation.
pub trait EventHandler: 'static + Send + Sync {
    /// Handles the occurrance of a trace.
    ///
    /// Typically, the handler would either show it to the user or write it to a file.
    ///
    /// # Arguments
    /// - `trace`: The [`Event`] to handle.
    ///
    /// # Errors
    /// This trace is allowed to error, but it should return it as a dynamic (`'static`) object.
    fn handle(&mut self, trace: Event) -> Result<(), Box<dyn 'static + Send + error::Error>>;
}

// Blanket impls
impl<T: EventHandler> EventHandler for Box<T> {
    #[inline]
    fn handle(&mut self, trace: Event) -> Result<(), Box<dyn 'static + Send + error::Error>> { <T as EventHandler>::handle(self, trace) }
}





/***** LIBRARY FUNCTIONS *****/
/// Registers a particular [`EventHandler`] such that it handles traces.
///
/// # Arguments
/// - `handler`: The [`EventHandler`] to register.
pub fn register_event_handler(handler: impl EventHandler) { let _ = EVENT_HANDLER.set(Mutex::new(Box::new(handler))); }





/***** LIBRARY *****/
/// Defines a catch-all wrapper for the sets in the prototype such that they produce nice traces.
pub struct TracingSet<T>(pub T);
// native impls
impl<E> TracingSet<MapAsync<E>>
where
    E: justact::Identifiable,
    E::Id: ToOwned,
{
    /// Registers a new agent.
    ///
    /// This will essentially create a new view for it. Be aware that the agent won't start to
    /// receive messages until it is registered.
    ///
    /// If the agent already existed, nothing happens.
    ///
    /// # Arguments
    /// - `id`: The ID of the agent to register.
    ///
    /// # Returns
    /// True if an agent already existed, or false otherwise.
    #[inline]
    pub fn register(&mut self, id: impl Into<String>) -> bool { self.0.register(id) }

    /// Returns a local view for a particular agent.
    ///
    /// # Arguments
    /// - `id`: The identifier of the agent to return the view for.
    ///
    /// # Returns
    /// A new [`MapAsyncView`] scoped to the agent with the given `id`.
    ///
    /// # Panics
    /// This function will panic if no agent with ID `id` is [registered](AsyncMap::register()).
    #[inline]
    pub fn scope<'s, 'i>(&'s mut self, id: &'i str) -> TracingSet<MapAsyncView<'s, 'i, E>> { TracingSet(self.0.scope(id)) }
}
// justact impls
impl<'s, 'i> justact::Map<Action> for TracingSet<MapAsyncView<'s, 'i, Action>> {
    type Error = Error<<MapAsyncView<'s, 'i, Action> as justact::Map<Action>>::Error>;

    #[inline]
    fn contains_key(&self, id: &<Action as justact::Identifiable>::Id) -> Result<bool, Self::Error>
    where
        Action: justact::Identifiable,
    {
        <MapAsyncView<'s, 'i, Action> as justact::Map<Action>>::contains_key(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn get(&self, id: &<Action as justact::Identifiable>::Id) -> Result<Option<&Action>, Self::Error>
    where
        Action: justact::Identifiable,
    {
        <MapAsyncView<'s, 'i, Action> as justact::Map<Action>>::get(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a Action>, Self::Error>
    where
        Action: 'a + justact::Identifiable,
    {
        <MapAsyncView<'s, 'i, Action> as justact::Map<Action>>::iter(&self.0).map_err(Error::Inner)
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { <MapAsyncView<'s, 'i, Action> as justact::Map<Action>>::len(&self.0).map_err(Error::Inner) }
}
impl<'s, 'i> justact::MapAsync<str, Action> for TracingSet<MapAsyncView<'s, 'i, Action>> {
    #[inline]
    fn add(&mut self, selector: justact::Recipient<&str>, elem: Action) -> Result<(), Self::Error>
    where
        Action: justact::Identifiable,
    {
        <MapAsyncView<'s, 'i, Action> as justact::MapAsync<str, Action>>::add(&mut self.0, selector.clone(), elem.clone()).map_err(Error::Inner)?;
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control(EventControl::EnactAction { who: Cow::Borrowed(self.0.id), to: selector.map(Cow::Borrowed), action: elem }))
            .map_err(|err| Error::EventHandle { err })?;
        Ok(())
    }
}
impl justact::Map<Agreement> for TracingSet<Agreements> {
    type Error = Error<<Agreements as justact::Map<Agreement>>::Error>;

    #[inline]
    fn contains_key(&self, id: &<Agreement as justact::Identifiable>::Id) -> Result<bool, Self::Error>
    where
        Agreement: justact::Identifiable,
    {
        <Agreements as justact::Map<Agreement>>::contains_key(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn get(&self, id: &<Agreement as justact::Identifiable>::Id) -> Result<Option<&Agreement>, Self::Error>
    where
        Agreement: justact::Identifiable,
    {
        <Agreements as justact::Map<Agreement>>::get(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s Agreement>, Self::Error>
    where
        Agreement: 's + justact::Identifiable,
    {
        <Agreements as justact::Map<Agreement>>::iter(&self.0).map_err(Error::Inner)
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { <Agreements as justact::Map<Agreement>>::len(&self.0).map_err(Error::Inner) }
}
impl justact::MapSync<Agreement> for TracingSet<Agreements> {
    #[inline]
    fn add(&mut self, elem: Agreement) -> Result<Option<Agreement>, Self::Error>
    where
        Agreement: justact::Identifiable,
    {
        let existing = <Agreements as justact::MapSync<Agreement>>::add(&mut self.0, elem.clone()).map_err(Error::Inner)?;
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control(EventControl::AddAgreement { agree: elem }))
            .map_err(|err| Error::EventHandle { err })?;
        Ok(existing)
    }
}
impl<'s, 'i> justact::Map<Arc<Message>> for TracingSet<MapAsyncView<'s, 'i, Arc<Message>>> {
    type Error = Error<<MapAsyncView<'s, 'i, Arc<Message>> as justact::Map<Arc<Message>>>::Error>;

    #[inline]
    fn contains_key(&self, id: &<Arc<Message> as justact::Identifiable>::Id) -> Result<bool, Self::Error>
    where
        Arc<Message>: justact::Identifiable,
    {
        <MapAsyncView<'s, 'i, Arc<Message>> as justact::Map<Arc<Message>>>::contains_key(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn get(&self, id: &<Arc<Message> as justact::Identifiable>::Id) -> Result<Option<&Arc<Message>>, Self::Error>
    where
        Arc<Message>: justact::Identifiable,
    {
        <MapAsyncView<'s, 'i, Arc<Message>> as justact::Map<Arc<Message>>>::get(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a Arc<Message>>, Self::Error>
    where
        Arc<Message>: 'a + justact::Identifiable,
    {
        <MapAsyncView<'s, 'i, Arc<Message>> as justact::Map<Arc<Message>>>::iter(&self.0).map_err(Error::Inner)
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> {
        <MapAsyncView<'s, 'i, Arc<Message>> as justact::Map<Arc<Message>>>::len(&self.0).map_err(Error::Inner)
    }
}
impl<'s, 'i> justact::MapAsync<str, Arc<Message>> for TracingSet<MapAsyncView<'s, 'i, Arc<Message>>> {
    #[inline]
    fn add(&mut self, selector: justact::Recipient<&str>, elem: Arc<Message>) -> Result<(), Self::Error>
    where
        Arc<Message>: justact::Identifiable,
    {
        <MapAsyncView<'s, 'i, Arc<Message>> as justact::MapAsync<str, Arc<Message>>>::add(&mut self.0, selector, elem.clone())
            .map_err(Error::Inner)?;
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control(EventControl::StateMessage { who: Cow::Borrowed(self.0.id), to: selector.map(Cow::Borrowed), msg: elem }))
            .map_err(|err| Error::EventHandle { err })?;
        Ok(())
    }
}
impl justact::Set<<Times as justact::Times>::Timestamp> for TracingSet<Times> {
    type Error = Error<<Times as justact::Set<<Times as justact::Times>::Timestamp>>::Error>;

    #[inline]
    fn contains(&self, elem: &<Times as justact::Times>::Timestamp) -> Result<bool, Self::Error> {
        <Times as justact::Set<<Times as justact::Times>::Timestamp>>::contains(&self.0, elem).map_err(Error::Inner)
    }

    #[inline]
    fn get(&self, elem: &<Times as justact::Times>::Timestamp) -> Result<Option<&<Times as justact::Times>::Timestamp>, Self::Error> {
        <Times as justact::Set<<Times as justact::Times>::Timestamp>>::get(&self.0, elem).map_err(Error::Inner)
    }

    #[inline]
    fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s <Times as justact::Times>::Timestamp>, Self::Error>
    where
        <Times as justact::Times>::Timestamp: 's,
    {
        <Times as justact::Set<<Times as justact::Times>::Timestamp>>::iter(&self.0).map_err(Error::Inner)
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { <Times as justact::Set<<Times as justact::Times>::Timestamp>>::len(&self.0).map_err(Error::Inner) }
}
impl justact::SetSync<<Times as justact::Times>::Timestamp> for TracingSet<Times> {
    #[inline]
    fn add(&mut self, elem: <Times as justact::Times>::Timestamp) -> Result<bool, Self::Error> {
        <Times as justact::SetSync<<Times as justact::Times>::Timestamp>>::add(&mut self.0, elem).map_err(Error::Inner)
    }
}
impl justact::Times for TracingSet<Times> {
    type Subset = <Times as justact::Times>::Subset;
    type Timestamp = <Times as justact::Times>::Timestamp;

    #[inline]
    fn current(&self) -> Result<Self::Subset, Self::Error> { <Times as justact::Times>::current(&self.0).map_err(Error::Inner) }
}
impl justact::TimesSync for TracingSet<Times> {
    #[inline]
    fn add_current(&mut self, timestamp: Self::Timestamp) -> Result<bool, Self::Error> {
        let existing = <Times as justact::TimesSync>::add_current(&mut self.0, timestamp).map_err(Error::Inner)?;
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control(EventControl::AdvanceTime { timestamp }))
            .map_err(|err| Error::EventHandle { err })?;
        Ok(existing)
    }
}
