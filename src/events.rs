//  EVENTS.rs
//    by Lut99
//
//  Created:
//    31 Jan 2025, 11:31:40
//  Last edited:
//    31 Jan 2025, 17:33:46
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a translation layer atop [`View`]s to create an
//!   event-trigger-like interface for agents.
//

use std::convert::Infallible;
use std::error::Error;
use std::marker::PhantomData;
use std::sync::Arc;
use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::collections::set::{Set, SetAsync, SetSync};
use justact::messages::ConstructableMessage;
use thiserror::Error;

#[cfg(feature = "dataplane")]
use crate::dataplane::ScopedStoreHandle;
use crate::wire::Message;


// /***** ERRORS *****/
// /// Extends [`Result<T, impl Error>`](Result) with the ability to be easily cast into an [`Error`].
// pub trait ResultToError<T> {
//     /// Casts this error into an [`Error`].
//     ///
//     /// # Returns
//     /// An [`Error`] that implements [`Error`](error::Error).
//     fn cast(self) -> Result<T, Error>;
// }
// impl<T, E: 'static + Send + error::Error> ResultToError<T> for Result<T, E> {
//     #[inline]
//     fn cast(self) -> Result<T, Error> { self.map_err(|err| Error(Box::new(err))) }
// }



/// A convenient error returned by the [`Composite`] datatype.
#[derive(Debug, Error)]
pub enum CompositeError<E1, E2> {
    #[error("{0}")]
    Left(E1),
    #[error("{0}")]
    Right(E2),
}

// /// An extremely generic error returned by the [`EventHandled`].
// #[derive(Debug)]
// pub struct Error(Box<dyn 'static + Send + error::Error>);
// impl Error {
//     /// Constructor for the Error from a generic [`Error`](error::Error).
//     ///
//     /// # Arguments
//     /// - `err`: The [`Error`](error::Error) to wrap.
//     ///
//     /// # Returns
//     /// A new Error, ready to wreak havoc.
//     #[inline]
//     #[allow(unused)]
//     pub fn new(err: impl 'static + Send + error::Error) -> Self { Self(Box::new(err)) }
// }
// impl Display for Error {
//     #[inline]
//     fn fmt(&self, f: &mut Formatter<'_>) -> FResult { self.0.fmt(f) }
// }
// impl error::Error for Error {
//     #[inline]
//     fn source(&self) -> Option<&(dyn error::Error + 'static)> { self.0.source() }
// }

/***** SCRIPT BUILDING BLOCKS *****/
/// Defines the abstract block.
pub trait ScriptBlock {
    type Error: Error;

    /// Clone of [`Agent::poll()`] with more convenient bounds for us.
    fn poll<A, S, E, MS>(&mut self, view: &mut View<A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        S: SetAsync<str, MS>;
}



/// Defines a building block for a script that publishes a statement the moment it is reached.
pub struct State<P: ?Sized + ToOwned> {
    /// The message to publish like, immediately.
    message: Option<Arc<Message<P>>>,
}
impl<P: ?Sized + ToOwned> State<P> {
    /// Constructor for the State script building block.
    ///
    /// # Arguments
    /// - `message`: Some [`Message`] to immediately publish when this scripting block is reached.
    ///
    /// # Returns
    /// A new State instance.
    #[inline]
    pub const fn new(message: Arc<Message<P>>) -> Self { Self { message: Some(message) } }
}
impl<P: ?Sized + ToOwned> ScriptBlock for State<P> {
    type Error = Infallible;

    #[inline]
    fn poll<A, S, E, MS>(&mut self, view: &mut View<A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        S: SetAsync<str, MS>,
    {
        if let Some(message) = self.message.take() {
            if let Err(err) = view.state() {}
        }

        // We're never waiting, so always "ready"
        Poll::Ready(Ok(()))
    }
}



/// Defines a compositional [`Handler`] that has some nice, custom syntax for building nodes.
pub struct Composite<H1, H2> {
    /// The thing to test
    left:  Option<H1>,
    /// The next one in the chain.
    right: Option<H2>,
}
impl<H1, H2> Composite<H1, H2> {
    /// Constructor for the Composite.
    ///
    /// # Arguments
    /// - `left`: The first handler to run.
    /// - `right`: The second handler to run. Its condition will only ever apply once the `left`
    ///   has had its condition meet.
    ///
    /// # Returns
    /// A new Composite that will first check for `H1`'s event, then check for `H2`'s event.
    #[inline]
    pub const fn new(left: H1, right: H2) -> Self { Self { left: Some(left), right: Some(right) } }
}
impl<C, H1: Handler<Context = C>, H2: Handler<Context = C>> Handler for Composite<H1, H2> {
    type Context = C;
    type Error = CompositeError<H1::Error, H2::Error>;

    #[inline]
    fn poll(&mut self, context: &mut Self::Context) -> Poll<Result<(), Self::Error>> {
        // First, poll the left handler, and yield it's result when any
        let res = if let Some(left) = &mut self.left { left.poll(context) } else { Poll::Pending };
        if let Poll::Ready(res) = res {
            self.left = None;
            return Poll::Ready(res.map_err(CompositeError::Left));
        }

        // Then, poll the other handler, doing the same thing.
        let res = if let Some(right) = &mut self.right { right.poll(context) } else { Poll::Pending };
        if let Poll::Ready(res) = res {
            self.right = None;
            Poll::Ready(res.map_err(CompositeError::Right))
        } else {
            Poll::Pending
        }
    }
}





/***** HANDLER BUILDER *****/
/// Convenient builder for building a script.
#[derive(Debug)]
pub struct ScriptBuilder<H>(H);
impl ScriptBuilder<()> {
    /// Constructor for the ScriptBuilder that initializes it as new.
    ///
    /// # Returns
    /// A ScriptBuilder with a phony handler `()` that doesn't really do anything.
    #[inline]
    pub const fn new() -> Self { Self(()) }
}
impl<H> ScriptBuilder<H> {
    /// Programs some action to run immediately when the script gets here.
    ///
    /// # Arguments
    /// - `closure`: The thing to do immediately.
    ///
    /// # Returns
    /// A ScriptBuilder that adds this step to the script.
    #[inline]
    pub fn run<F: FnOnce(&mut A) -> Result<(), E>, A, E>(self, closure: F) -> ScriptBuilder<Composite<H, Immediate<F, A, E>>> {
        ScriptBuilder(Composite::new(self.0, Immediate::new(closure)))
    }



    /// Completes the script, returning a [`Handler`] that you can [`Handler::poll()`].
    ///
    /// # Returns
    /// The inner `H`andler, ready to be polled.
    #[inline]
    pub fn finish(self) -> H { self.0 }
}




/***** LIBRARY *****/





// /***** LIBRARY *****/
// /// Implements a translation layer atop a [`View`] such that agents can write their scripts as
// /// event handlers.
// pub struct EventHandler {
//     /// Whether or not we did the first one.
//     on_start_handled: bool,
//     /// The set of agreements & statements for which we check collectively.
//     on_agreed_and_stated_handled: HashSet<((String, u32), Vec<(String, u32)>)>,
//     /// The set of agreements that we've already handled.
//     on_agreed_handled: HashSet<(String, u32)>,
//     /// The set of statements that we've already handled.
//     on_stated_handled: HashSet<(String, u32)>,
//     /// The set of facts that we've already handled.
//     on_truth_handled: HashSet<GroundAtom>,
//     /// The set of fact sets that we've already handled.
//     on_truths_handled: HashSet<Vec<GroundAtom>>,
//     /// The set of enactments that we've already handled.
//     on_enacted_handled: HashSet<(String, char)>,
//     /// The set of data creations (writes) we've already handled.
//     #[cfg(feature = "dataplane")]
//     on_data_created_handled: HashSet<((String, String), String)>,
//     /// The set of sets of data creations (writes) we've already handled.
//     #[cfg(feature = "dataplane")]
//     on_datas_created_handled: HashSet<Vec<((String, String), String)>>,
//     /// The set of agreements & sets of data creations (writes) we've already handled.
//     #[cfg(feature = "dataplane")]
//     on_enacted_and_datas_created_handled: HashSet<((String, char), Vec<((String, String), String)>)>,
// }
// impl Default for EventHandler {
//     #[inline]
//     fn default() -> Self { Self::new() }
// }
// impl EventHandler {
//     /// Constructor for the EventHandler that initializes it as new.
//     ///
//     /// # Returns
//     /// A new EventHandler that can be used to (wait for it) handle events.
//     #[inline]
//     pub fn new() -> Self {
//         Self {
//             on_start_handled: false,
//             on_agreed_and_stated_handled: HashSet::new(),
//             on_agreed_handled: HashSet::new(),
//             on_stated_handled: HashSet::new(),
//             on_truth_handled: HashSet::new(),
//             on_truths_handled: HashSet::new(),
//             on_enacted_handled: HashSet::new(),
//             #[cfg(feature = "dataplane")]
//             on_data_created_handled: HashSet::new(),
//             #[cfg(feature = "dataplane")]
//             on_datas_created_handled: HashSet::new(),
//             #[cfg(feature = "dataplane")]
//             on_enacted_and_datas_created_handled: HashSet::new(),
//         }
//     }
// }
// impl EventHandler {
//     /// Processes events in the given [`View`] and returns an [`EventHandled`] that can be used to
//     /// run triggers.
//     ///
//     /// # Arguments
//     /// - `view`: Some [`View`] to process.
//     ///
//     /// # Returns
//     /// An [`EventHandled`] that can be used to process triggers.
//     #[inline]
//     pub const fn handle<A, S, E>(&mut self, view: View<A, S, E>) -> EventHandled<'_, A, S, E> {
//         EventHandled {
//             handler: self,
//             view,
//             #[cfg(feature = "dataplane")]
//             store: None,
//             ready: true,
//         }
//     }

//     /// Processes events in the given [`View`] and [`ScopedStoreHandle`] and returns an
//     /// [`EventHandled`] that can be used to run triggers.
//     ///
//     /// # Arguments
//     /// - `view`: Some [`View`] to process.
//     /// - `handle`: Some kind of [`StoreHandle`] that is used to trigger on dataplane events.
//     ///
//     /// # Returns
//     /// An [`EventHandled`] that can be used to process triggers.
//     #[inline]
//     #[cfg(feature = "dataplane")]
//     pub const fn handle_with_store<A, S, E>(&mut self, view: View<A, S, E>, store: ScopedStoreHandle) -> EventHandled<'_, A, S, E> {
//         EventHandled { handler: self, view, store: Some(store), ready: true }
//     }
// }

// /// Implements the state of the [`EventHandler`] after a [`View`] is added.
// pub struct EventHandled<'h, A, S, E> {
//     /// The handler (that contains some state).
//     handler: &'h mut EventHandler,
//     /// The view to process.
//     view:    View<A, S, E>,
//     /// The store to use for dataplane access. Since not all agents need that, it's optional.
//     #[cfg(feature = "dataplane")]
//     store:   Option<ScopedStoreHandle>,
//     /// Whether all handles registered for this view are triggered (i.e., the agent is done).
//     ready:   bool,
// }
// impl<'h, A, S, E> EventHandled<'h, A, S, E> {
//     /// Adds a new handler for when the scenario starts.
//     ///
//     /// # Arguments
//     /// - `closure`: Some [`FnOnce`] that will be executed initially.
//     ///
//     /// # Returns
//     /// Self for chaining.
//     ///
//     /// # Errors
//     /// This function may error if the closure errors.
//     pub fn on_start<ERR>(mut self, closure: impl FnOnce(&mut View<A, S, E>) -> Result<(), ERR>) -> Result<Self, Error>
//     where
//         ERR: 'static + Send + error::Error,
//     {
//         // Don't run it if already started, lol
//         if self.handler.on_start_handled {
//             return Ok(self);
//         }

//         // Simply run the closure
//         self.handler.on_start_handled = true;
//         closure(&mut self.view).cast()?;
//         Ok(self)
//     }

//     /// Adds a new handler for when a new fact has become available to this agent.
//     ///
//     /// # Arguments
//     /// - `fact`: The fact to wait for.
//     /// - `closure`: Some [`FnOnce`] that will be executed when the fact has become available.
//     ///
//     /// # Returns
//     /// Self for chaining.
//     ///
//     /// # Errors
//     /// This function may error if something went wrong with interacting with the sets in the
//     /// internal view.
//     pub fn on_truth<SM, ERR>(mut self, fact: GroundAtom, closure: impl FnOnce(&mut View<A, S, E>) -> Result<(), ERR>) -> Result<Self, Error>
//     where
//         S: Set<SM>,
//         SM: Message<AuthorId = str, Payload = Program>,
//         ERR: 'static + Send + error::Error,
//     {
//         // Don't do anything if we've already handled it
//         if self.handler.on_truth_handled.contains(&fact) {
//             return Ok(self);
//         }

//         // Else, consider messages until it is there
//         let mut found: bool = false;
//         for msg in self.view.stated.iter().cast()? {
//             // Compute the denotation of this message
//             let set = Singleton::new(msg);
//             let truths: Denotation = Extractor.extract(&set).cast()?.truths();
//             if <Denotation as InfallibleSet<GroundAtom>>::contains(&truths, &fact) {
//                 found = true;
//                 break;
//             }
//         }

//         // Handle the case we found it (we do it here for borrowing-of-view purposes)
//         if found {
//             // Handled it
//             self.handler.on_truth_handled.insert(fact);

//             // Call the closure
//             closure(&mut self.view).cast()?;
//         } else {
//             self.ready = false;
//         }

//         // Done
//         Ok(self)
//     }

//     /// Adds a new handler for when a specific set of facts has become available to this agent.
//     ///
//     /// # Arguments
//     /// - `facts`: The set of facts to wait for.
//     /// - `closure`: Some [`FnOnce`] that will be executed when the facts have become available.
//     ///
//     /// # Returns
//     /// Self for chaining.
//     ///
//     /// # Errors
//     /// This function may error if something went wrong with interacting with the sets in the
//     /// internal view.
//     pub fn on_truths<SM, ERR>(
//         mut self,
//         facts: impl IntoIterator<Item = GroundAtom>,
//         closure: impl FnOnce(&mut View<A, S, E>) -> Result<(), ERR>,
//     ) -> Result<Self, Error>
//     where
//         S: Set<SM>,
//         SM: Message<AuthorId = str, Payload = Program>,
//         ERR: 'static + Send + error::Error,
//     {
//         let facts: Vec<GroundAtom> = facts.into_iter().collect();

//         // Don't do anything if we've already handled it
//         if self.handler.on_truths_handled.contains(&facts) {
//             return Ok(self);
//         }

//         // Else, consider messages until it is there
//         for fact in &facts {
//             let mut found: bool = false;
//             for msg in self.view.stated.iter().cast()? {
//                 // Compute the denotation of this message
//                 let set = Singleton::new(msg);
//                 let truths: Denotation = Extractor.extract(&set).cast()?.truths();
//                 if <Denotation as InfallibleSet<GroundAtom>>::contains(&truths, &fact) {
//                     found = true;
//                     break;
//                 }
//             }
//             if !found {
//                 self.ready = false;
//                 return Ok(self);
//             }
//         }

//         // Call the closure
//         self.handler.on_truths_handled.insert(facts);
//         closure(&mut self.view).cast()?;

//         // Done
//         Ok(self)
//     }

//     /// Triggers on a certain dataset existing.
//     ///
//     /// # Arguments
//     /// - `id`: The identifier of the dataset to wait for.
//     /// - `closure`: Some [`FnOnce`] that will be triggered once the dataset exists.
//     ///
//     /// # Returns
//     /// Self for chaining.
//     ///
//     /// # Errors
//     /// This function may error if something went wrong with interacting with the sets in the
//     /// internal view.
//     #[cfg(feature = "dataplane")]
//     #[inline]
//     pub fn on_data_created<ERR>(
//         mut self,
//         id: ((impl Into<String>, impl Into<String>), impl Into<String>),
//         closure: impl FnOnce(&mut View<A, S, E>) -> Result<(), ERR>,
//     ) -> Result<Self, Error>
//     where
//         ERR: 'static + Send + error::Error,
//     {
//         let store: &ScopedStoreHandle =
//             self.store.as_ref().unwrap_or_else(|| panic!("Cannot call EventHandled::on_data_creates() if not handling with a store"));
//         let id: ((String, String), String) = ((id.0.0.into(), id.0.1.into()), id.1.into());

//         // Don't do anything if we've already handled it
//         if self.handler.on_data_created_handled.contains(&id) {
//             return Ok(self);
//         }

//         // Otherwise, check the internal handle for when it's there
//         if store.exists(&id) {
//             self.handler.on_data_created_handled.insert(id);
//             closure(&mut self.view).cast()?;
//         } else {
//             self.ready = false;
//         }
//         Ok(self)
//     }

//     /// Triggers on a set of certain datasets existing.
//     ///
//     /// # Arguments
//     /// - `ids`: The identifiers of the datasets to wait for.
//     /// - `closure`: Some [`FnOnce`] that will be triggered once the dataset exists.
//     ///
//     /// # Returns
//     /// Self for chaining.
//     ///
//     /// # Errors
//     /// This function may error if something went wrong with interacting with the sets in the
//     /// internal view.
//     #[cfg(feature = "dataplane")]
//     #[inline]
//     pub fn on_datas_created<ERR>(
//         mut self,
//         ids: impl IntoIterator<Item = ((impl Into<String>, impl Into<String>), impl Into<String>)>,
//         closure: impl FnOnce(&mut View<A, S, E>) -> Result<(), ERR>,
//     ) -> Result<Self, Error>
//     where
//         ERR: 'static + Send + error::Error,
//     {
//         let store: &ScopedStoreHandle =
//             self.store.as_ref().unwrap_or_else(|| panic!("Cannot call EventHandled::on_data_creates() if not handling with a store"));
//         let ids: Vec<((String, String), String)> = ids.into_iter().map(|id| ((id.0.0.into(), id.0.1.into()), id.1.into())).collect();

//         // Don't do anything if we've already handled it
//         if self.handler.on_datas_created_handled.contains(&ids) {
//             return Ok(self);
//         }

//         // Otherwise, check the internal handle for when it's there
//         for id in &ids {
//             if !store.exists(&id) {
//                 self.ready = false;
//                 return Ok(self);
//             }
//         }

//         // They all exist at this point!
//         self.handler.on_datas_created_handled.insert(ids);
//         closure(&mut self.view).cast()?;
//         Ok(self)
//     }

//     /// Finishes the handling.
//     ///
//     /// Basically just examines whether all called triggers are actually triggered (or have been in
//     /// the past), which means the agent can die happily.
//     ///
//     /// # Returns
//     /// [`Poll::Ready`] when all triggers (have been) triggered, or [`Poll::Pending`] otherwise.
//     ///
//     /// # Errors
//     /// This function actually never errors. Just here to be more convenient to agents.
//     #[inline]
//     pub fn finish(self) -> Result<Poll<()>, Error> { if self.ready { Ok(Poll::Ready(())) } else { Ok(Poll::Pending) } }
// }
