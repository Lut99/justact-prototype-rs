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
use std::fmt::{Debug, Display, Formatter, Result as FResult};
use std::hash::Hash;
use std::sync::{Arc, Mutex, OnceLock};

use crate::auditing::{Event, EventControl};
use crate::policy::PolicySerialize;
use crate::sets::{Agreements, SetAsync, SetAsyncView};
use crate::wire::{Action, Message};

mod justact {
    pub use ::justact::auxillary::Identifiable;
    pub use ::justact::collections::Recipient;
    pub use ::justact::collections::set::{Set, SetAsync, SetSync};
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
    fn handle(&mut self, trace: Event<str>) -> Result<(), Box<dyn 'static + Send + error::Error>>;
}

// Blanket impls
impl<T: EventHandler> EventHandler for Box<T> {
    #[inline]
    fn handle(&mut self, trace: Event<str>) -> Result<(), Box<dyn 'static + Send + error::Error>> { <T as EventHandler>::handle(self, trace) }
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
impl<E> TracingSet<SetAsync<E>>
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
    /// A new [`SetAsyncView`] scoped to the agent with the given `id`.
    ///
    /// # Panics
    /// This function will panic if no agent with ID `id` is [registered](AsyncSet::register()).
    #[inline]
    pub fn scope<'s, 'i>(&'s mut self, id: &'i str) -> TracingSet<SetAsyncView<'s, 'i, E>> { TracingSet(self.0.scope(id)) }
}
// justact impls
impl<'s, 'i, P: ?Sized + ToOwned> justact::Set<Action<P>> for TracingSet<SetAsyncView<'s, 'i, Action<P>>>
where
    P: 'static,
    P::Owned: 'static + Debug + Eq + Hash + Send + Sync,
    Action<P>: justact::Identifiable<Id = (String, char)>,
{
    type Error = Error<<SetAsyncView<'s, 'i, Action<P>> as justact::Set<Action<P>>>::Error>;

    #[inline]
    fn get(&self, elem: &Action<P>) -> Result<Option<&Action<P>>, Self::Error> {
        <SetAsyncView<'s, 'i, Action<P>> as justact::Set<Action<P>>>::get(&self.0, elem).map_err(Error::Inner)
    }

    #[inline]
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a Action<P>>, Self::Error>
    where
        Action<P>: 'a,
    {
        <SetAsyncView<'s, 'i, Action<P>> as justact::Set<Action<P>>>::iter(&self.0).map_err(Error::Inner)
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { <SetAsyncView<'s, 'i, Action<P>> as justact::Set<Action<P>>>::len(&self.0).map_err(Error::Inner) }
}
impl<'s, 'i, P: ?Sized + PolicySerialize + ToOwned> justact::SetAsync<str, Action<P>> for TracingSet<SetAsyncView<'s, 'i, Action<P>>>
where
    P: 'static,
    P::Owned: 'static + Clone + Debug + Eq + Hash + Send + Sync,
    Action<P>: justact::Identifiable<Id = (String, char)>,
{
    #[inline]
    fn add(&mut self, selector: justact::Recipient<&str>, elem: Action<P>) -> Result<(), Self::Error> {
        <SetAsyncView<'s, 'i, Action<P>> as justact::SetAsync<str, Action<P>>>::add(&mut self.0, selector.clone(), elem.clone())
            .map_err(Error::Inner)?;
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control {
                event: EventControl::EnactAction::<str> {
                    who:    Cow::Borrowed(self.0.id),
                    to:     selector.map(Cow::Borrowed),
                    action: elem.serialize(),
                },
            })
            .map_err(|err| Error::EventHandle { err })?;
        Ok(())
    }
}
impl<P: ?Sized + ToOwned> justact::Set<Arc<Message<P>>> for TracingSet<Agreements<P>>
where
    P: 'static,
    P::Owned: 'static + Debug + Eq + Hash + Send + Sync,
{
    type Error = Error<<Agreements<P> as justact::Set<Arc<Message<P>>>>::Error>;

    #[inline]
    fn get(&self, elem: &Arc<Message<P>>) -> Result<Option<&Arc<Message<P>>>, Self::Error> {
        <Agreements<P> as justact::Set<Arc<Message<P>>>>::get(&self.0, elem).map_err(Error::Inner)
    }

    #[inline]
    fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s Arc<Message<P>>>, Self::Error>
    where
        Arc<Message<P>>: 's,
    {
        <Agreements<P> as justact::Set<Arc<Message<P>>>>::iter(&self.0).map_err(Error::Inner)
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { <Agreements<P> as justact::Set<Arc<Message<P>>>>::len(&self.0).map_err(Error::Inner) }
}
impl<P: ?Sized + PolicySerialize + ToOwned> justact::SetSync<Arc<Message<P>>> for TracingSet<Agreements<P>>
where
    P: 'static,
    P::Owned: 'static + Debug + Eq + Hash + Send + Sync,
{
    #[inline]
    fn add(&mut self, elem: Arc<Message<P>>) -> Result<bool, Self::Error> {
        let existing = <Agreements<P> as justact::SetSync<Arc<Message<P>>>>::add(&mut self.0, elem.clone()).map_err(Error::Inner)?;
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control { event: EventControl::AddAgreement { agree: Arc::new(elem.serialize()) } })
            .map_err(|err| Error::EventHandle { err })?;
        Ok(existing)
    }
}
impl<'s, 'i, P: ?Sized + ToOwned> justact::Set<Arc<Message<P>>> for TracingSet<SetAsyncView<'s, 'i, Arc<Message<P>>>>
where
    P: 'static,
    P::Owned: 'static + Debug + Eq + Hash + Send + Sync,
    Arc<Message<P>>: justact::Identifiable<Id = (String, u32)>,
{
    type Error = Error<<SetAsyncView<'s, 'i, Arc<Message<P>>> as justact::Set<Arc<Message<P>>>>::Error>;

    #[inline]
    fn get(&self, elem: &Arc<Message<P>>) -> Result<Option<&Arc<Message<P>>>, Self::Error> {
        <SetAsyncView<'s, 'i, Arc<Message<P>>> as justact::Set<Arc<Message<P>>>>::get(&self.0, elem).map_err(Error::Inner)
    }

    #[inline]
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a Arc<Message<P>>>, Self::Error>
    where
        Arc<Message<P>>: 'a + justact::Identifiable,
    {
        <SetAsyncView<'s, 'i, Arc<Message<P>>> as justact::Set<Arc<Message<P>>>>::iter(&self.0).map_err(Error::Inner)
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> {
        <SetAsyncView<'s, 'i, Arc<Message<P>>> as justact::Set<Arc<Message<P>>>>::len(&self.0).map_err(Error::Inner)
    }
}
impl<'s, 'i, P: ?Sized + PolicySerialize + ToOwned> justact::SetAsync<str, Arc<Message<P>>> for TracingSet<SetAsyncView<'s, 'i, Arc<Message<P>>>>
where
    P: 'static,
    P::Owned: 'static + Debug + Eq + Hash + Send + Sync,
    Arc<Message<P>>: justact::Identifiable<Id = (String, u32)>,
{
    #[inline]
    fn add(&mut self, selector: justact::Recipient<&str>, elem: Arc<Message<P>>) -> Result<(), Self::Error> {
        <SetAsyncView<'s, 'i, Arc<Message<P>>> as justact::SetAsync<str, Arc<Message<P>>>>::add(&mut self.0, selector, elem.clone())
            .map_err(Error::Inner)?;
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control {
                event: EventControl::StateMessage {
                    who: Cow::Borrowed(self.0.id),
                    to:  selector.map(Cow::Borrowed),
                    msg: Arc::new(elem.serialize()),
                },
            })
            .map_err(|err| Error::EventHandle { err })?;
        Ok(())
    }
}
