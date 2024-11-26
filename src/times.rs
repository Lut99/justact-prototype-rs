//  TIMES.rs
//    by Lut99
//
//  Created:
//    23 May 2024, 17:36:27
//  Last edited:
//    26 Nov 2024, 11:51:39
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements (various) global views on timestamps.
//

use std::cell::RefCell;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::rc::Rc;

use justact::times::{Times as JATimes, Timestamp};

use crate::interface::Interface;


/***** ERRORS *****/
/// Determines the possible errors for the [`TimesDicatator`] set.
#[derive(Debug)]
pub enum TimesDictatorError {
    /// The agent attempting to advance the time was not the dictator.
    NotTheDictator { agent: String, dictator: String },
}
impl Display for TimesDictatorError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use TimesDictatorError::*;
        match self {
            NotTheDictator { agent, dictator } => {
                write!(f, "Agent '{agent}' failed to advance the time because they are not the dictator ('{dictator}' is)")
            },
        }
    }
}
impl Error for TimesDictatorError {}





/***** LIBRARY *****/
/// An owned version of the times.
///
/// This variation synchronizes new times if and only if it's a particular agent claiming it.
///
/// Agents will see the agent-scoped variation [`TimesDictator`].
#[derive(Debug)]
pub struct GlobalTimesDictator {
    /// The only agent allowed to make changes.
    dictator:  String,
    /// An interface we use to log whatever happens in pretty ways.
    interface: Rc<RefCell<Interface>>,
    /// The current timestamp.
    current:   Timestamp,
}
impl GlobalTimesDictator {
    /// Constructor for the GlobalTimesDictator.
    ///
    /// # Agreements
    /// - `dictator`: The agent who will get to decide everything.
    /// - `interface`: An interface we use to log whatever happens in pretty ways.
    ///
    /// # Returns
    /// A new GlobalTimesDictator.
    #[inline]
    pub fn new(dictator: impl Into<String>, interface: Rc<RefCell<Interface>>) -> Self {
        let dictator: String = dictator.into();
        Self { dictator: dictator.clone(), interface, current: Timestamp(0) }
    }

    /// Allows an agent scoped access to the Times-set.
    ///
    /// # Arguments
    /// - `agent`: The agent to scope this [`GlobalTimesDictator`] for.
    /// - `func`: Some function that is executed for this scope.
    ///
    /// # Returns
    /// A new [`TimesDictator`] that implements [`justact_core::agreements::Agreements`].
    #[inline]
    pub fn scope<R>(&mut self, agent: &str, func: impl FnOnce(&mut TimesDictator) -> R) -> R {
        // Call the closure
        let mut view = TimesDictator { agent, dictator: &self.dictator, current: self.current, queue: vec![] };
        let res: R = func(&mut view);

        // Sync the changes back
        if let Some(current) = view.queue.pop() {
            self.current = current;
            self.interface.borrow().log_advance(current);
        }

        // OK, done
        res
    }
}
impl JATimes for GlobalTimesDictator {
    type Error = TimesDictatorError;

    #[inline]
    fn current(&self) -> Timestamp { self.current }

    #[inline]
    fn advance_to(&mut self, timestamp: Timestamp) -> Result<(), Self::Error> {
        // Do not advance if we're not the dictator
        if self.dictator == "<system>" {
            self.current = timestamp;
            Ok(())
        } else {
            Err(TimesDictatorError::NotTheDictator { agent: "<system>".into(), dictator: self.dictator.clone() })
        }
    }
}

/// Provides agents with a global view on the current time.
///
/// This variation synchronizes time if and only if it's a particular agent claiming it.
#[derive(Debug)]
pub struct TimesDictator<'v> {
    /// This agent
    agent:    &'v str,
    /// The only agent allowed to make changes.
    dictator: &'v str,

    /// The statements that this agent knows of.
    current: Timestamp,
    /// A queue of statements that this agent pushed.
    pub(crate) queue: Vec<Timestamp>,
}
impl<'v> JATimes for TimesDictator<'v> {
    type Error = TimesDictatorError;

    #[inline]
    fn current(&self) -> Timestamp {
        // See if the agent pushed a more recent one; else, take the one at creation time
        if let Some(time) = self.queue.last() { *time } else { self.current }
    }

    #[inline]
    fn advance_to(&mut self, timestamp: Timestamp) -> Result<(), Self::Error> {
        // Do not advance if we're not the dictator
        if self.agent == self.dictator {
            self.queue.push(timestamp);
            Ok(())
        } else {
            Err(TimesDictatorError::NotTheDictator { agent: self.agent.into(), dictator: self.dictator.into() })
        }
    }
}
impl<'t, 'v> JATimes for &'t mut TimesDictator<'v> {
    type Error = <TimesDictator<'v> as JATimes>::Error;

    #[inline]
    fn current(&self) -> Timestamp { TimesDictator::current(self) }

    #[inline]
    fn advance_to(&mut self, timestamp: Timestamp) -> Result<(), Self::Error> { TimesDictator::advance_to(self, timestamp) }
}
