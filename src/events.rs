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

use std::collections::HashSet;
use std::error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::task::Poll;

use justact::actors::View;
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Singleton;
use justact::collections::map::Map;
use justact::collections::set::InfallibleSet;
use justact::messages::Message;
use justact::policies::{Extractor as _, Policy as _};
use justact::times::Times;
use slick::GroundAtom;

#[cfg(feature = "dataplane")]
use crate::dataplane::ScopedStoreHandle;
use crate::policy::slick::{Denotation, Extractor};


/***** ERRORS *****/
/// Extends [`Result<T, impl Error>`](Result) with the ability to be easily cast into an [`Error`].
pub trait ResultToError<T> {
    /// Casts this error into an [`Error`].
    ///
    /// # Returns
    /// An [`Error`] that implements [`Error`](error::Error).
    fn cast(self) -> Result<T, Error>;
}
impl<T, E: 'static + Send + error::Error> ResultToError<T> for Result<T, E> {
    #[inline]
    fn cast(self) -> Result<T, Error> { self.map_err(|err| Error(Box::new(err))) }
}



/// An extremely generic error returned by the [`EventHandled`].
#[derive(Debug)]
pub struct Error(Box<dyn 'static + Send + error::Error>);
impl Error {
    /// Constructor for the Error from a generic [`Error`](error::Error).
    ///
    /// # Arguments
    /// - `err`: The [`Error`](error::Error) to wrap.
    ///
    /// # Returns
    /// A new Error, ready to wreak havoc.
    #[inline]
    #[allow(unused)]
    pub fn new(err: impl 'static + Send + error::Error) -> Self { Self(Box::new(err)) }
}
impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult { self.0.fmt(f) }
}
impl error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> { self.0.source() }
}





/***** LIBRARY *****/
/// Implements a translation layer atop a [`View`] such that agents can write their scripts as
/// event handlers.
pub struct EventHandler {
    /// Whether or not we did the first one.
    on_start_handled: bool,
    /// The set of agreements & statements for which we check collectively.
    on_agreed_and_stated_handled: HashSet<((String, u32), Vec<(String, u32)>)>,
    /// The set of agreements that we've already handled.
    on_agreed_handled: HashSet<(String, u32)>,
    /// The set of statements that we've already handled.
    on_stated_handled: HashSet<(String, u32)>,
    /// The set of facts that we've already handled.
    on_truth_handled: HashSet<GroundAtom>,
    /// The set of fact sets that we've already handled.
    on_truths_handled: HashSet<Vec<GroundAtom>>,
    /// The set of enactments that we've already handled.
    on_enacted_handled: HashSet<(String, char)>,
    /// The set of times has been updated with a specific current one.
    on_tick_to_handled: HashSet<u64>,
    /// The set of data creations (writes) we've already handled.
    #[cfg(feature = "dataplane")]
    on_data_created_handled: HashSet<((String, String), String)>,
    /// The set of sets of data creations (writes) we've already handled.
    #[cfg(feature = "dataplane")]
    on_datas_created_handled: HashSet<Vec<((String, String), String)>>,
    /// The set of agreements & sets of data creations (writes) we've already handled.
    #[cfg(feature = "dataplane")]
    on_enacted_and_datas_created_handled: HashSet<((String, char), Vec<((String, String), String)>)>,
}
impl Default for EventHandler {
    #[inline]
    fn default() -> Self { Self::new() }
}
impl EventHandler {
    /// Constructor for the EventHandler that initializes it as new.
    ///
    /// # Returns
    /// A new EventHandler that can be used to (wait for it) handle events.
    #[inline]
    pub fn new() -> Self {
        Self {
            on_start_handled: false,
            on_agreed_and_stated_handled: HashSet::new(),
            on_agreed_handled: HashSet::new(),
            on_stated_handled: HashSet::new(),
            on_truth_handled: HashSet::new(),
            on_truths_handled: HashSet::new(),
            on_enacted_handled: HashSet::new(),
            on_tick_to_handled: HashSet::new(),
            #[cfg(feature = "dataplane")]
            on_data_created_handled: HashSet::new(),
            #[cfg(feature = "dataplane")]
            on_datas_created_handled: HashSet::new(),
            #[cfg(feature = "dataplane")]
            on_enacted_and_datas_created_handled: HashSet::new(),
        }
    }
}
impl EventHandler {
    /// Processes events in the given [`View`] and returns an [`EventHandled`] that can be used to
    /// run triggers.
    ///
    /// # Arguments
    /// - `view`: Some [`View`] to process.
    ///
    /// # Returns
    /// An [`EventHandled`] that can be used to process triggers.
    #[inline]
    pub const fn handle<T, A, S, E>(&mut self, view: View<T, A, S, E>) -> EventHandled<T, A, S, E> {
        EventHandled {
            handler: self,
            view,
            #[cfg(feature = "dataplane")]
            store: None,
            ready: true,
        }
    }

    /// Processes events in the given [`View`] and [`ScopedStoreHandle`] and returns an
    /// [`EventHandled`] that can be used to run triggers.
    ///
    /// # Arguments
    /// - `view`: Some [`View`] to process.
    /// - `handle`: Some kind of [`StoreHandle`] that is used to trigger on dataplane events.
    ///
    /// # Returns
    /// An [`EventHandled`] that can be used to process triggers.
    #[inline]
    #[cfg(feature = "dataplane")]
    pub const fn handle_with_store<T, A, S, E>(&mut self, view: View<T, A, S, E>, store: ScopedStoreHandle) -> EventHandled<T, A, S, E> {
        EventHandled { handler: self, view, store: Some(store), ready: true }
    }
}

/// Implements the state of the [`EventHandler`] after a [`View`] is added.
pub struct EventHandled<'h, T, A, S, E> {
    /// The handler (that contains some state).
    handler: &'h mut EventHandler,
    /// The view to process.
    view:    View<T, A, S, E>,
    /// The store to use for dataplane access. Since not all agents need that, it's optional.
    #[cfg(feature = "dataplane")]
    store:   Option<ScopedStoreHandle>,
    /// Whether all handles registered for this view are triggered (i.e., the agent is done).
    ready:   bool,
}
impl<'h, T, A, S, E> EventHandled<'h, T, A, S, E> {
    /// Adds a new handler for when the scenario starts.
    ///
    /// # Arguments
    /// - `closure`: Some [`FnOnce`] that will be executed initially.
    ///
    /// # Returns
    /// Self for chaining.
    ///
    /// # Errors
    /// This function may error if the closure errors.
    pub fn on_start<ERR>(mut self, closure: impl FnOnce(&mut View<T, A, S, E>) -> Result<(), ERR>) -> Result<Self, Error>
    where
        ERR: 'static + Send + error::Error,
    {
        // Don't run it if already started, lol
        if self.handler.on_start_handled {
            return Ok(self);
        }

        // Simply run the closure
        self.handler.on_start_handled = true;
        closure(&mut self.view).cast()?;
        Ok(self)
    }

    /// Adds a new handler for when a particular agreement is active and a set of messages have
    /// been stated.
    ///
    /// This is useful before publishing enactments.
    ///
    /// # Arguments
    /// - `agree`: The ID of the agreements to wait for.
    /// - `stmts`: A set of identifiers to of statements to wait for.
    /// - `closure`: Some [`FnOnce`] that will be executed when the conditions are met.
    ///
    /// # Returns
    /// Self for chaining.
    ///
    /// # Errors
    /// This function may error if something went wrong with interacting with the sets in the
    /// internal view.
    pub fn on_agreed_and_stated<SM, ERR>(
        mut self,
        agree: (impl Into<String>, u32),
        stmts: impl IntoIterator<Item = (impl Into<String>, u32)>,
        closure: impl FnOnce(&mut View<T, A, S, E>, Agreement<SM, u64>, Vec<SM>) -> Result<(), ERR>,
    ) -> Result<Self, Error>
    where
        T: Times<Timestamp = u64>,
        A: Map<Agreement<SM, u64>>,
        S: Map<SM>,
        SM: Clone + Identifiable<Id = (String, u32)>,
        ERR: 'static + Send + error::Error,
    {
        let agree: (String, u32) = (agree.0.into(), agree.1);
        let stmts: Vec<(String, u32)> = stmts.into_iter().map(|(a, i)| (a.into(), i)).collect();

        // Don't do anything if we've already handled it
        if self.handler.on_agreed_and_stated_handled.contains(&(agree.clone(), stmts.clone())) {
            return Ok(self);
        }

        // First, wait until the agreement is there
        let found_agree: Agreement<SM, u64> = if let Some(agree) = self.view.agreed.get(&agree).cast()? {
            // Holdup; ensure the agreement's time is current too!
            if self.view.times.current().cast()?.contains(&agree.at) {
                agree.clone()
            } else {
                self.ready = false;
                return Ok(self);
            }
        } else {
            self.ready = false;
            return Ok(self);
        };

        // Then, we wait until all messages are here
        let mut found_stmts: Vec<SM> = Vec::new();
        for stmt in &stmts {
            if let Some(msg) = self.view.stated.get(&stmt).cast()?.cloned() {
                found_stmts.push(msg);
            } else {
                self.ready = false;
                return Ok(self);
            }
        }
        if found_stmts.len() != stmts.len() {
            self.ready = false;
            return Ok(self);
        }

        // Now we can run the closure
        self.handler.on_agreed_and_stated_handled.insert((agree, stmts));
        closure(&mut self.view, found_agree, found_stmts).cast()?;

        // Done
        Ok(self)
    }

    /// Adds a new handler for when a new agreement is received by this agent AND marked as active.
    ///
    /// # Arguments
    /// - `id`: The ID of the message to wait for.
    /// - `closure`: Some [`FnOnce`] that will be executed when the statement has become available.
    ///
    /// # Returns
    /// Self for chaining.
    ///
    /// # Errors
    /// This function may error if something went wrong with interacting with the sets in the
    /// internal view.
    pub fn on_agreed<SM, ERR>(
        mut self,
        id: (impl Into<String>, u32),
        closure: impl FnOnce(&mut View<T, A, S, E>, Agreement<SM, u64>) -> Result<(), ERR>,
    ) -> Result<Self, Error>
    where
        T: Times<Timestamp = u64>,
        A: Map<Agreement<SM, u64>>,
        SM: Clone + Identifiable<Id = (String, u32)>,
        ERR: 'static + Send + error::Error,
    {
        let id: (String, u32) = (id.0.into(), id.1);

        // Don't do anything if we've already handled it
        if self.handler.on_agreed_handled.contains(&id) {
            return Ok(self);
        }

        // Else, check to see if the statement has become available
        let agree: Option<Agreement<SM, u64>> = if let Some(agree) = self.view.agreed.get(&id).cast()? {
            // Holdup; ensure the agreement's time is current too!
            if self.view.times.current().cast()?.contains(&agree.at) { Some(agree.clone()) } else { None }
        } else {
            None
        };

        // If it has, call the closure!
        // (We do it here for borrowing purposes)
        if let Some(agree) = agree {
            // Handled it
            self.handler.on_agreed_handled.insert(id);

            // Call the closure
            closure(&mut self.view, agree.clone()).cast()?;
        } else {
            self.ready = false;
        }

        // Done
        Ok(self)
    }

    /// Adds a new handler for when a new statement is received by this agent.
    ///
    /// # Arguments
    /// - `id`: The ID of the message to wait for.
    /// - `closure`: Some [`FnOnce`] that will be executed when the statement has become available.
    ///
    /// # Returns
    /// Self for chaining.
    ///
    /// # Errors
    /// This function may error if something went wrong with interacting with the sets in the
    /// internal view.
    pub fn on_stated<SM, ERR>(
        mut self,
        id: (impl Into<String>, u32),
        closure: impl FnOnce(&mut View<T, A, S, E>, SM) -> Result<(), ERR>,
    ) -> Result<Self, Error>
    where
        S: Map<SM>,
        SM: Clone + Identifiable<Id = (String, u32)>,
        ERR: 'static + Send + error::Error,
    {
        let id: (String, u32) = (id.0.into(), id.1);

        // Don't do anything if we've already handled it
        if self.handler.on_stated_handled.contains(&id) {
            return Ok(self);
        }

        // Else, check to see if the statement has become available
        if let Some(msg) = self.view.stated.get(&id).cast()?.cloned() {
            // Handled it
            self.handler.on_stated_handled.insert(id);

            // Call the closure
            closure(&mut self.view, msg).cast()?;
        } else {
            self.ready = false;
        }

        // Done
        Ok(self)
    }

    /// Adds a new handler for when a new fact has become available to this agent.
    ///
    /// # Arguments
    /// - `fact`: The fact to wait for.
    /// - `closure`: Some [`FnOnce`] that will be executed when the fact has become available.
    ///
    /// # Returns
    /// Self for chaining.
    ///
    /// # Errors
    /// This function may error if something went wrong with interacting with the sets in the
    /// internal view.
    pub fn on_truth<SM, ERR>(mut self, fact: GroundAtom, closure: impl FnOnce(&mut View<T, A, S, E>) -> Result<(), ERR>) -> Result<Self, Error>
    where
        S: Map<SM>,
        SM: Message<Id = (String, u32), AuthorId = str, Payload = str>,
        ERR: 'static + Send + error::Error,
    {
        // Don't do anything if we've already handled it
        if self.handler.on_truth_handled.contains(&fact) {
            return Ok(self);
        }

        // Else, consider messages until it is there
        let mut found: bool = false;
        for msg in self.view.stated.iter().cast()? {
            // Compute the denotation of this message
            let set = Singleton::new(msg);
            let truths: Denotation = Extractor.extract(&set).cast()?.truths();
            if <Denotation as InfallibleSet<GroundAtom>>::contains(&truths, &fact) {
                found = true;
                break;
            }
        }

        // Handle the case we found it (we do it here for borrowing-of-view purposes)
        if found {
            // Handled it
            self.handler.on_truth_handled.insert(fact);

            // Call the closure
            closure(&mut self.view).cast()?;
        } else {
            self.ready = false;
        }

        // Done
        Ok(self)
    }

    /// Adds a new handler for when a specific set of facts has become available to this agent.
    ///
    /// # Arguments
    /// - `facts`: The set of facts to wait for.
    /// - `closure`: Some [`FnOnce`] that will be executed when the facts have become available.
    ///
    /// # Returns
    /// Self for chaining.
    ///
    /// # Errors
    /// This function may error if something went wrong with interacting with the sets in the
    /// internal view.
    pub fn on_truths<SM, ERR>(
        mut self,
        facts: impl IntoIterator<Item = GroundAtom>,
        closure: impl FnOnce(&mut View<T, A, S, E>) -> Result<(), ERR>,
    ) -> Result<Self, Error>
    where
        S: Map<SM>,
        SM: Message<Id = (String, u32), AuthorId = str, Payload = str>,
        ERR: 'static + Send + error::Error,
    {
        let facts: Vec<GroundAtom> = facts.into_iter().collect();

        // Don't do anything if we've already handled it
        if self.handler.on_truths_handled.contains(&facts) {
            return Ok(self);
        }

        // Else, consider messages until it is there
        for fact in &facts {
            let mut found: bool = false;
            for msg in self.view.stated.iter().cast()? {
                // Compute the denotation of this message
                let set = Singleton::new(msg);
                let truths: Denotation = Extractor.extract(&set).cast()?.truths();
                if <Denotation as InfallibleSet<GroundAtom>>::contains(&truths, &fact) {
                    found = true;
                    break;
                }
            }
            if !found {
                self.ready = false;
                return Ok(self);
            }
        }

        // Call the closure
        self.handler.on_truths_handled.insert(facts);
        closure(&mut self.view).cast()?;

        // Done
        Ok(self)
    }

    /// Adds a new handler for when a new action is received by this agent.
    ///
    /// # Arguments
    /// - `id`: The ID of the action to wait for.
    /// - `closure`: Some [`FnOnce`] that will be executed when the action has become available.
    ///
    /// # Returns
    /// Self for chaining.
    ///
    /// # Errors
    /// This function may error if something went wrong with interacting with the sets in the
    /// internal view.
    pub fn on_enacted<SA, ERR>(
        mut self,
        id: (impl Into<String>, char),
        closure: impl FnOnce(&mut View<T, A, S, E>, SA) -> Result<(), ERR>,
    ) -> Result<Self, Error>
    where
        E: Map<SA>,
        SA: Clone + Identifiable<Id = (String, char)>,
        ERR: 'static + Send + error::Error,
    {
        let id: (String, char) = (id.0.into(), id.1);

        // Don't do anything if we've already handled it
        if self.handler.on_enacted_handled.contains(&id) {
            return Ok(self);
        }

        // Else, check to see if the statement has become available
        if let Some(act) = self.view.enacted.get(&id).cast()?.cloned() {
            // Handled it
            self.handler.on_enacted_handled.insert(id);

            // Call the closure
            closure(&mut self.view, act).cast()?;
        } else {
            self.ready = false;
        }

        // Done
        Ok(self)
    }

    /// Adds a new handler for when the current time is updated to the given one.
    ///
    /// # Arguments
    /// - `tiemstamp`: The timestamp to wait for.
    /// - `closure`: Some [`FnOnce`] that will be executed when the action has become available.
    ///
    /// # Returns
    /// Self for chaining.
    ///
    /// # Errors
    /// This function may error if something went wrong with interacting with the sets in the
    /// internal view.
    pub fn on_tick_to<ERR>(mut self, timestamp: u64, closure: impl FnOnce(&mut View<T, A, S, E>) -> Result<(), ERR>) -> Result<Self, Error>
    where
        T: Times<Timestamp = u64>,
        ERR: 'static + Send + error::Error,
    {
        // Don't do anything if we've already handled it
        if self.handler.on_tick_to_handled.contains(&timestamp) {
            return Ok(self);
        }

        // Else, check to see if the statement has become available
        if self.view.times.current().cast()?.contains(&timestamp) {
            // Handled it
            self.handler.on_tick_to_handled.insert(timestamp);

            // Call the closure
            closure(&mut self.view).cast()?;
        } else {
            self.ready = false;
        }

        // Done
        Ok(self)
    }

    /// Triggers on a certain dataset existing.
    ///
    /// # Arguments
    /// - `id`: The identifier of the dataset to wait for.
    /// - `closure`: Some [`FnOnce`] that will be triggered once the dataset exists.
    ///
    /// # Returns
    /// Self for chaining.
    ///
    /// # Errors
    /// This function may error if something went wrong with interacting with the sets in the
    /// internal view.
    #[cfg(feature = "dataplane")]
    #[inline]
    pub fn on_data_created<ERR>(
        mut self,
        id: ((impl Into<String>, impl Into<String>), impl Into<String>),
        closure: impl FnOnce(&mut View<T, A, S, E>) -> Result<(), ERR>,
    ) -> Result<Self, Error>
    where
        ERR: 'static + Send + error::Error,
    {
        let store: &ScopedStoreHandle =
            self.store.as_ref().unwrap_or_else(|| panic!("Cannot call EventHandled::on_data_creates() if not handling with a store"));
        let id: ((String, String), String) = ((id.0.0.into(), id.0.1.into()), id.1.into());

        // Don't do anything if we've already handled it
        if self.handler.on_data_created_handled.contains(&id) {
            return Ok(self);
        }

        // Otherwise, check the internal handle for when it's there
        if store.exists(&id) {
            self.handler.on_data_created_handled.insert(id);
            closure(&mut self.view).cast()?;
        } else {
            self.ready = false;
        }
        Ok(self)
    }

    /// Triggers on a set of certain datasets existing.
    ///
    /// # Arguments
    /// - `ids`: The identifiers of the datasets to wait for.
    /// - `closure`: Some [`FnOnce`] that will be triggered once the dataset exists.
    ///
    /// # Returns
    /// Self for chaining.
    ///
    /// # Errors
    /// This function may error if something went wrong with interacting with the sets in the
    /// internal view.
    #[cfg(feature = "dataplane")]
    #[inline]
    pub fn on_datas_created<ERR>(
        mut self,
        ids: impl IntoIterator<Item = ((impl Into<String>, impl Into<String>), impl Into<String>)>,
        closure: impl FnOnce(&mut View<T, A, S, E>) -> Result<(), ERR>,
    ) -> Result<Self, Error>
    where
        ERR: 'static + Send + error::Error,
    {
        let store: &ScopedStoreHandle =
            self.store.as_ref().unwrap_or_else(|| panic!("Cannot call EventHandled::on_data_creates() if not handling with a store"));
        let ids: Vec<((String, String), String)> = ids.into_iter().map(|id| ((id.0.0.into(), id.0.1.into()), id.1.into())).collect();

        // Don't do anything if we've already handled it
        if self.handler.on_datas_created_handled.contains(&ids) {
            return Ok(self);
        }

        // Otherwise, check the internal handle for when it's there
        for id in &ids {
            if !store.exists(&id) {
                self.ready = false;
                return Ok(self);
            }
        }

        // They all exist at this point!
        self.handler.on_datas_created_handled.insert(ids);
        closure(&mut self.view).cast()?;
        Ok(self)
    }

    /// Triggers on an action being enacted and a set of certain datasets existing.
    ///
    /// # Arguments
    /// - `act`: The identifier of the action to wait for.
    /// - `datas`: The identifiers of the datasets to wait for.
    /// - `closure`: Some [`FnOnce`] that will be triggered once the dataset exists.
    ///
    /// # Returns
    /// Self for chaining.
    ///
    /// # Errors
    /// This function may error if something went wrong with interacting with the sets in the
    /// internal view.
    #[cfg(feature = "dataplane")]
    #[inline]
    pub fn on_enacted_and_datas_created<SA, ERR>(
        mut self,
        act: (impl Into<String>, char),
        datas: impl IntoIterator<Item = ((impl Into<String>, impl Into<String>), impl Into<String>)>,
        closure: impl FnOnce(&mut View<T, A, S, E>, SA) -> Result<(), ERR>,
    ) -> Result<Self, Error>
    where
        E: Map<SA>,
        SA: Clone + Identifiable<Id = (String, char)>,
        ERR: 'static + Send + error::Error,
    {
        let store: &ScopedStoreHandle =
            self.store.as_ref().unwrap_or_else(|| panic!("Cannot call EventHandled::on_data_creates() if not handling with a store"));
        let act: (String, char) = (act.0.into(), act.1);
        let datas: Vec<((String, String), String)> = datas.into_iter().map(|id| ((id.0.0.into(), id.0.1.into()), id.1.into())).collect();

        // Don't do anything if we've already handled it
        if self.handler.on_enacted_and_datas_created_handled.contains(&(act.clone(), datas.clone())) {
            return Ok(self);
        }

        // Then check for the agreement
        let found_act: SA = match self.view.enacted.get(&act).cast()?.cloned() {
            Some(act) => act,
            None => {
                self.ready = false;
                return Ok(self);
            },
        };

        // And check the internal handle for when it's there
        for id in &datas {
            if !store.exists(&id) {
                self.ready = false;
                return Ok(self);
            }
        }

        // They all exist at this point!
        self.handler.on_enacted_and_datas_created_handled.insert((act, datas));
        closure(&mut self.view, found_act).cast()?;
        Ok(self)
    }

    /// Finishes the handling.
    ///
    /// Basically just examines whether all called triggers are actually triggered (or have been in
    /// the past), which means the agent can die happily.
    ///
    /// # Returns
    /// [`Poll::Ready`] when all triggers (have been) triggered, or [`Poll::Pending`] otherwise.
    ///
    /// # Errors
    /// This function actually never errors. Just here to be more convenient to agents.
    #[inline]
    pub fn finish(self) -> Result<Poll<()>, Error> { if self.ready { Ok(Poll::Ready(())) } else { Ok(Poll::Pending) } }
}
