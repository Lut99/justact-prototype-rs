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

use ::justact::collections::set::InfallibleSet as _;
use slick::Program;

use crate::auditing::{Event, EventControl};
use crate::wire::{Message, into_prototype_action, into_prototype_message};

mod justact {
    pub use ::justact::actions::ConstructableAction;
    pub use ::justact::actors::View;
    pub use ::justact::collections::Recipient;
    pub use ::justact::collections::set::{SetAsync, SetSync};
    pub use ::justact::messages::ConstructableMessage;
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
/// Defines a wrapper around a [`View`](justact::View) that will log what happens to it.
pub struct TracingView<'v, A, S, E>(pub &'v mut justact::View<str, A, S, E>);

/// Implement the view update functions for our impl
impl<'v, 's, 'i, A, S, E> TracingView<'v, A, S, E> {
    /// Have the agent state a message to their own view.
    ///
    /// # Arguments
    /// - `msg`: The message to state.
    ///
    /// # Errors
    /// This function errors if adding the message to the internal `S`tatements-set fails to add
    /// the new message, or if the agent attempted to publish a message not theirs.
    #[inline]
    pub fn state<SM>(&mut self, msg: SM) -> Result<(), Error<::justact::actors::Error<String, S::Error>>>
    where
        S: justact::SetAsync<str, SM>,
        SM: justact::ConstructableMessage<AuthorId = str, Payload = Program>,
    {
        // State the message, first
        let pmsg = into_prototype_message(&msg);
        self.0.state(msg).map_err(Error::Inner)?;

        // Then log that it happened
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control {
                event: EventControl::StateMessage {
                    who: Cow::Borrowed(&self.0.id),
                    to:  justact::Recipient::One(Cow::Borrowed(&self.0.id)),
                    msg: pmsg,
                },
            })
            .map_err(|err| Error::EventHandle { err })
    }

    /// Have the agent enact an action to their own view.
    ///
    /// # Arguments
    /// - `act`: The action to enact.
    ///
    /// # Errors
    /// This function errors if adding the message to the internal `S`tatements-set fails to add
    /// the new message.
    #[inline]
    pub fn enact<SM, SA>(&mut self, act: SA) -> Result<(), Error<::justact::actors::Error<String, E::Error>>>
    where
        E: justact::SetAsync<str, SA>,
        SM: justact::ConstructableMessage<AuthorId = str, Payload = Program>,
        SA: justact::ConstructableAction<ActorId = str, Message = SM>,
    {
        // Enact the action, first
        let pact = into_prototype_action(&act);
        self.0.enact(act).map_err(Error::Inner)?;

        // Then log that it happened
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control {
                event: EventControl::EnactAction {
                    who:    Cow::Borrowed(&self.0.id),
                    to:     justact::Recipient::One(Cow::Borrowed(&self.0.id)),
                    action: pact,
                },
            })
            .map_err(|err| Error::EventHandle { err })
    }

    /// Agree on a new agreement.
    ///
    /// Specifically, replaces all of the agreements with the given list.
    ///
    /// # Arguments
    /// - `agrees`: An iterator yielding agreements to put in the list.
    ///
    /// # Errors
    /// This function errors if we failed to clear the existing list or if we added any of the
    /// agreements yielded by `agrees`.
    #[inline]
    pub fn agree<SM>(&mut self, agrees: impl IntoIterator<Item = SM>) -> Result<(), Error<::justact::actors::Error<String, A::Error>>>
    where
        A: justact::SetSync<SM>,
        SM: justact::ConstructableMessage<AuthorId = str, Payload = Program>,
    {
        // Update the agreements, first
        let agrees: Vec<SM> = agrees.into_iter().collect();
        let pagrees: Vec<Arc<Message<str>>> = agrees.iter().map(into_prototype_message).collect();
        self.0.agree(agrees).map_err(Error::Inner)?;

        // Then log that it happened
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control { event: EventControl::SetAgreements { agrees: pagrees } })
            .map_err(|err| Error::EventHandle { err })
    }



    /// Gossips a particular message to a new recipient.
    ///
    /// Note, though, that the message must already be in the agent's view for this to be allowed.
    ///
    /// # Arguments
    /// - `to`: Some [`Recipient`] to gossip the message to.
    /// - `message`: The message to gossip.
    ///
    /// # Errors
    /// This function errors if we failed to access the list of stated messages or if the current
    /// agent did not know the `message`.
    #[inline]
    pub fn gossip<SM>(&mut self, to: justact::Recipient<String>, message: SM) -> Result<(), Error<::justact::actors::Error<String, S::Error>>>
    where
        S: justact::SetAsync<str, SM>,
        SM: justact::ConstructableMessage<AuthorId = str, Payload = Program>,
    {
        // Gossip first
        let pmsg = into_prototype_message(&message);
        self.0.gossip(to.clone(), message).map_err(Error::Inner)?;

        // Then log that it happened
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Control {
                event: EventControl::StateMessage {
                    who: Cow::Borrowed(&self.0.id),
                    to:  match to {
                        justact::Recipient::All => justact::Recipient::All,
                        justact::Recipient::One(id) => justact::Recipient::One(Cow::Owned(id)),
                    },
                    msg: pmsg,
                },
            })
            .map_err(|err| Error::EventHandle { err })
    }
}



// /// Defines a catch-all wrapper for the sets in the prototype such that they produce nice traces.
// pub struct TracingSet<T>(pub T);
// // native impls
// impl<E> TracingSet<SetAsync<E>> {
//     /// Registers a new agent.
//     ///
//     /// This will essentially create a new view for it. Be aware that the agent won't start to
//     /// receive messages until it is registered.
//     ///
//     /// If the agent already existed, nothing happens.
//     ///
//     /// # Arguments
//     /// - `id`: The ID of the agent to register.
//     ///
//     /// # Returns
//     /// True if an agent already existed, or false otherwise.
//     #[inline]
//     pub fn register(&mut self, id: impl Into<String>) -> bool { self.0.register(id) }

//     /// Returns a local view for a particular agent.
//     ///
//     /// # Arguments
//     /// - `id`: The identifier of the agent to return the view for.
//     ///
//     /// # Returns
//     /// A new [`SetAsyncView`] scoped to the agent with the given `id`.
//     ///
//     /// # Panics
//     /// This function will panic if no agent with ID `id` is [registered](AsyncSet::register()).
//     #[inline]
//     pub fn scope<'s, 'i>(&'s mut self, id: &'i str) -> TracingSet<SetAsyncView<'s, 'i, E>> { TracingSet(self.0.scope(id)) }
// }
// // justact impls
// impl<'s, 'i, P: ?Sized + ToOwned> justact::Set<Action<P>> for TracingSet<SetAsyncView<'s, 'i, Action<P>>>
// where
//     P: 'static,
//     P::Owned: 'static + Debug + Eq + Hash + Send + Sync,
// {
//     type Error = Error<<SetAsyncView<'s, 'i, Action<P>> as justact::Set<Action<P>>>::Error>;

//     #[inline]
//     fn get(&self, elem: &Action<P>) -> Result<Option<&Action<P>>, Self::Error> {
//         <SetAsyncView<'s, 'i, Action<P>> as justact::Set<Action<P>>>::get(&self.0, elem).map_err(Error::Inner)
//     }

//     #[inline]
//     fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a Action<P>>, Self::Error>
//     where
//         Action<P>: 'a,
//     {
//         <SetAsyncView<'s, 'i, Action<P>> as justact::Set<Action<P>>>::iter(&self.0).map_err(Error::Inner)
//     }

//     #[inline]
//     fn len(&self) -> Result<usize, Self::Error> { <SetAsyncView<'s, 'i, Action<P>> as justact::Set<Action<P>>>::len(&self.0).map_err(Error::Inner) }
// }
// impl<'s, 'i, P: ?Sized + PolicySerialize + ToOwned> justact::SetAsync<str, Action<P>> for TracingSet<SetAsyncView<'s, 'i, Action<P>>>
// where
//     P: 'static,
//     P::Owned: 'static + Clone + Debug + Eq + Hash + Send + Sync,
// {
//     #[inline]
//     fn add(&mut self, selector: justact::Recipient<&str>, elem: Action<P>) -> Result<(), Self::Error> {
//         <SetAsyncView<'s, 'i, Action<P>> as justact::SetAsync<str, Action<P>>>::add(&mut self.0, selector.clone(), elem.clone())
//             .map_err(Error::Inner)?;
//         EVENT_HANDLER
//             .get()
//             .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
//             .lock()
//             .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
//             .handle(Event::Control {
//                 event: EventControl::EnactAction::<str> {
//                     who:    Cow::Borrowed(self.0.id),
//                     to:     selector.map(Cow::Borrowed),
//                     action: elem.serialize(),
//                 },
//             })
//             .map_err(|err| Error::EventHandle { err })?;
//         Ok(())
//     }
// }
// impl<P: ?Sized + ToOwned> justact::Set<Arc<Message<P>>> for TracingSet<Agreements<P>>
// where
//     P: 'static,
//     P::Owned: 'static + Debug + Eq + Hash + Send + Sync,
// {
//     type Error = Error<<Agreements<P> as justact::Set<Arc<Message<P>>>>::Error>;

//     #[inline]
//     fn get(&self, elem: &Arc<Message<P>>) -> Result<Option<&Arc<Message<P>>>, Self::Error> {
//         <Agreements<P> as justact::Set<Arc<Message<P>>>>::get(&self.0, elem).map_err(Error::Inner)
//     }

//     #[inline]
//     fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s Arc<Message<P>>>, Self::Error>
//     where
//         Arc<Message<P>>: 's,
//     {
//         <Agreements<P> as justact::Set<Arc<Message<P>>>>::iter(&self.0).map_err(Error::Inner)
//     }

//     #[inline]
//     fn len(&self) -> Result<usize, Self::Error> { <Agreements<P> as justact::Set<Arc<Message<P>>>>::len(&self.0).map_err(Error::Inner) }
// }
// impl<P: ?Sized + PolicySerialize + ToOwned> justact::SetSync<Arc<Message<P>>> for TracingSet<Agreements<P>>
// where
//     P: 'static,
//     P::Owned: 'static + Debug + Eq + Hash + Send + Sync,
// {
//     #[inline]
//     fn add(&mut self, elem: Arc<Message<P>>) -> Result<bool, Self::Error> {
//         let existing = <Agreements<P> as justact::SetSync<Arc<Message<P>>>>::add(&mut self.0, elem.clone()).map_err(Error::Inner)?;
//         EVENT_HANDLER
//             .get()
//             .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
//             .lock()
//             .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
//             .handle(Event::Control { event: EventControl::AddAgreement { agree: Arc::new(elem.serialize()) } })
//             .map_err(|err| Error::EventHandle { err })?;
//         Ok(existing)
//     }

//     #[inline]
//     fn clear(&mut self) -> Result<(), Self::Error> {
//         EVENT_HANDLER
//             .get()
//             .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
//             .lock()
//             .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
//             .handle(Event::Control { event: EventControl:: })
//     }
// }
// impl<'s, 'i, P: ?Sized + ToOwned> justact::Set<Arc<Message<P>>> for TracingSet<SetAsyncView<'s, 'i, Arc<Message<P>>>>
// where
//     P: 'static,
//     P::Owned: 'static + Debug + Eq + Hash + Send + Sync,
// {
//     type Error = Error<<SetAsyncView<'s, 'i, Arc<Message<P>>> as justact::Set<Arc<Message<P>>>>::Error>;

//     #[inline]
//     fn get(&self, elem: &Arc<Message<P>>) -> Result<Option<&Arc<Message<P>>>, Self::Error> {
//         <SetAsyncView<'s, 'i, Arc<Message<P>>> as justact::Set<Arc<Message<P>>>>::get(&self.0, elem).map_err(Error::Inner)
//     }

//     #[inline]
//     fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a Arc<Message<P>>>, Self::Error>
//     where
//         Arc<Message<P>>: 'a,
//     {
//         <SetAsyncView<'s, 'i, Arc<Message<P>>> as justact::Set<Arc<Message<P>>>>::iter(&self.0).map_err(Error::Inner)
//     }

//     #[inline]
//     fn len(&self) -> Result<usize, Self::Error> {
//         <SetAsyncView<'s, 'i, Arc<Message<P>>> as justact::Set<Arc<Message<P>>>>::len(&self.0).map_err(Error::Inner)
//     }
// }
// impl<'s, 'i, P: ?Sized + PolicySerialize + ToOwned> justact::SetAsync<str, Arc<Message<P>>> for TracingSet<SetAsyncView<'s, 'i, Arc<Message<P>>>>
// where
//     P: 'static,
//     P::Owned: 'static + Debug + Eq + Hash + Send + Sync,
// {
//     #[inline]
//     fn add(&mut self, selector: justact::Recipient<&str>, elem: Arc<Message<P>>) -> Result<(), Self::Error> {
//         <SetAsyncView<'s, 'i, Arc<Message<P>>> as justact::SetAsync<str, Arc<Message<P>>>>::add(&mut self.0, selector, elem.clone())
//             .map_err(Error::Inner)?;
//         EVENT_HANDLER
//             .get()
//             .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
//             .lock()
//             .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
//             .handle(Event::Control {
//                 event: EventControl::StateMessage {
//                     who: Cow::Borrowed(self.0.id),
//                     to:  selector.map(Cow::Borrowed),
//                     msg: Arc::new(elem.serialize()),
//                 },
//             })
//             .map_err(|err| Error::EventHandle { err })?;
//         Ok(())
//     }
// }
