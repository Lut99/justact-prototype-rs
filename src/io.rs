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
use std::sync::{Arc, Mutex, OnceLock};

use crate::auditing::{Event, EventControl};
use crate::policy::PolicySerialize;
use crate::sets::{Agreements, MapAsync, MapAsyncView, Times};
use crate::wire::{Action, Agreement, Message, serialize_agreement};

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
impl<'s, 'i, P: ?Sized + ToOwned> justact::Map<Action<P>> for TracingSet<MapAsyncView<'s, 'i, Action<P>>>
where
    Action<P>: justact::Identifiable<Id = (String, char)>,
{
    type Error = Error<<MapAsyncView<'s, 'i, Action<P>> as justact::Map<Action<P>>>::Error>;

    #[inline]
    fn contains_key(&self, id: &<Action<P> as justact::Identifiable>::Id) -> Result<bool, Self::Error> {
        <MapAsyncView<'s, 'i, Action<P>> as justact::Map<Action<P>>>::contains_key(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn get(&self, id: &<Action<P> as justact::Identifiable>::Id) -> Result<Option<&Action<P>>, Self::Error> {
        <MapAsyncView<'s, 'i, Action<P>> as justact::Map<Action<P>>>::get(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a Action<P>>, Self::Error>
    where
        Action<P>: 'a,
    {
        <MapAsyncView<'s, 'i, Action<P>> as justact::Map<Action<P>>>::iter(&self.0).map_err(Error::Inner)
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { <MapAsyncView<'s, 'i, Action<P>> as justact::Map<Action<P>>>::len(&self.0).map_err(Error::Inner) }
}
impl<'s, 'i, P: ?Sized + PolicySerialize + ToOwned> justact::MapAsync<str, Action<P>> for TracingSet<MapAsyncView<'s, 'i, Action<P>>>
where
    P::Owned: Clone,
    Action<P>: justact::Identifiable<Id = (String, char)>,
{
    #[inline]
    fn add(&mut self, selector: justact::Recipient<&str>, elem: Action<P>) -> Result<(), Self::Error> {
        <MapAsyncView<'s, 'i, Action<P>> as justact::MapAsync<str, Action<P>>>::add(&mut self.0, selector.clone(), elem.clone())
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
impl<P: ?Sized + ToOwned> justact::Map<Agreement<P>> for TracingSet<Agreements<P>>
where
    Agreement<P>: justact::Identifiable<Id = (String, u32)>,
{
    type Error = Error<<Agreements<P> as justact::Map<Agreement<P>>>::Error>;

    #[inline]
    fn contains_key(&self, id: &<Agreement<P> as justact::Identifiable>::Id) -> Result<bool, Self::Error> {
        <Agreements<P> as justact::Map<Agreement<P>>>::contains_key(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn get(&self, id: &<Agreement<P> as justact::Identifiable>::Id) -> Result<Option<&Agreement<P>>, Self::Error> {
        <Agreements<P> as justact::Map<Agreement<P>>>::get(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s Agreement<P>>, Self::Error>
    where
        Agreement<P>: 's,
    {
        <Agreements<P> as justact::Map<Agreement<P>>>::iter(&self.0).map_err(Error::Inner)
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { <Agreements<P> as justact::Map<Agreement<P>>>::len(&self.0).map_err(Error::Inner) }
}
impl<P: ?Sized + PolicySerialize + ToOwned> justact::MapSync<Agreement<P>> for TracingSet<Agreements<P>>
where
    Agreement<P>: justact::Identifiable<Id = (String, u32)>,
{
    #[inline]
    fn add(&mut self, elem: Agreement<P>) -> Result<Option<Agreement<P>>, Self::Error> {
        let existing = <Agreements<P> as justact::MapSync<Agreement<P>>>::add(&mut self.0, elem.clone()).map_err(Error::Inner)?;
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control { event: EventControl::AddAgreement { agree: serialize_agreement(&elem) } })
            .map_err(|err| Error::EventHandle { err })?;
        Ok(existing)
    }
}
impl<'s, 'i, P: ?Sized + ToOwned> justact::Map<Arc<Message<P>>> for TracingSet<MapAsyncView<'s, 'i, Arc<Message<P>>>>
where
    Arc<Message<P>>: justact::Identifiable<Id = (String, u32)>,
{
    type Error = Error<<MapAsyncView<'s, 'i, Arc<Message<P>>> as justact::Map<Arc<Message<P>>>>::Error>;

    #[inline]
    fn contains_key(&self, id: &<Arc<Message<P>> as justact::Identifiable>::Id) -> Result<bool, Self::Error> {
        <MapAsyncView<'s, 'i, Arc<Message<P>>> as justact::Map<Arc<Message<P>>>>::contains_key(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn get(&self, id: &<Arc<Message<P>> as justact::Identifiable>::Id) -> Result<Option<&Arc<Message<P>>>, Self::Error> {
        <MapAsyncView<'s, 'i, Arc<Message<P>>> as justact::Map<Arc<Message<P>>>>::get(&self.0, id).map_err(Error::Inner)
    }

    #[inline]
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a Arc<Message<P>>>, Self::Error>
    where
        Arc<Message<P>>: 'a + justact::Identifiable,
    {
        <MapAsyncView<'s, 'i, Arc<Message<P>>> as justact::Map<Arc<Message<P>>>>::iter(&self.0).map_err(Error::Inner)
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> {
        <MapAsyncView<'s, 'i, Arc<Message<P>>> as justact::Map<Arc<Message<P>>>>::len(&self.0).map_err(Error::Inner)
    }
}
impl<'s, 'i, P: ?Sized + PolicySerialize + ToOwned> justact::MapAsync<str, Arc<Message<P>>> for TracingSet<MapAsyncView<'s, 'i, Arc<Message<P>>>>
where
    Arc<Message<P>>: justact::Identifiable<Id = (String, u32)>,
{
    #[inline]
    fn add(&mut self, selector: justact::Recipient<&str>, elem: Arc<Message<P>>) -> Result<(), Self::Error> {
        <MapAsyncView<'s, 'i, Arc<Message<P>>> as justact::MapAsync<str, Arc<Message<P>>>>::add(&mut self.0, selector, elem.clone())
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
            .handle(Event::Control { event: EventControl::AdvanceTime { timestamp } })
            .map_err(|err| Error::EventHandle { err })?;
        Ok(existing)
    }
}
