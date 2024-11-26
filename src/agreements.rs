//  AGREEMENTS.rs
//    by Lut99
//
//  Created:
//    23 May 2024, 17:42:56
//  Last edited:
//    26 Nov 2024, 11:49:51
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements (various) global views on agreements.
//

use std::cell::RefCell;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::rc::Rc;

use justact::agreements::{Agreement, Agreements as JAAgreements};
use justact::auxillary::Identifiable as _;
use justact::set::LocalSet;

use crate::interface::Interface;
use crate::statements::Message;


/***** ERRORS *****/
/// Determines the possible errors for the [`AgreementsDictator`] set.
#[derive(Debug)]
pub enum AgreementsDictatorError {
    /// The agent attempting to advance the time was not the dictator.
    NotTheDictator { id: String, agent: String, dictator: String },
}
impl Display for AgreementsDictatorError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use AgreementsDictatorError::*;
        match self {
            NotTheDictator { id, agent, dictator } => {
                write!(f, "Agent '{agent}' failed to create an agreement out of statement '{id}' because they are not the dictator ('{dictator}' is)")
            },
        }
    }
}
impl Error for AgreementsDictatorError {}





/***** LIBRARY *****/
/// An owned version of the agreements.
///
/// This variation synchronizes new agreements if and only if it's a particular agent claiming it.
///
/// Agents will see the agent-scoped variation [`AgreementsDictator`].
#[derive(Debug)]
pub struct GlobalAgreementsDictator {
    /// The only agent allowed to make changes.
    dictator:  String,
    /// An interface we use to log whatever happens in pretty ways.
    interface: Rc<RefCell<Interface>>,
    /// All agreements in the land.
    agrs:      LocalSet<Agreement<Message>>,
}
impl GlobalAgreementsDictator {
    /// Constructor for the GlobalAgreementsDictator.
    ///
    /// # Agreements
    /// - `dictator`: The agent who will get to decide everything.
    /// - `interface`: An interface we use to log whatever happens in pretty ways.
    ///
    /// # Returns
    /// A new GlobalAgreementsDictator.
    #[inline]
    pub fn new(dictator: impl Into<String>, interface: Rc<RefCell<Interface>>) -> Self {
        let dictator: String = dictator.into();
        Self { dictator: dictator.clone(), interface, agrs: LocalSet::new() }
    }

    /// Returns an [`AgreementsDictator`] which is scoped for a particular agent.
    ///
    /// # Arguments
    /// - `agent`: The agent to scope this [`GlobalAgreementsDictator`] for.
    /// - `func`: Some function that is executed for this scope.
    ///
    /// # Returns
    /// A new [`AgreementsDictator`] that implements [`justact_core::agreements::Agreements`].
    #[inline]
    pub fn scope<R>(&mut self, agent: &str, func: impl FnOnce(&mut AgreementsDictator) -> R) -> R {
        // Call the closure
        let (res, mut queue): (R, Vec<Agreement<Message>>) = {
            let mut view = AgreementsDictator { agent, dictator: &self.dictator, agrs: &self.agrs, queue: vec![] };
            let res: R = func(&mut view);
            (res, view.queue)
        };

        // Sync the changes back
        self.agrs.reserve(queue.len());
        for agr in queue.drain(..) {
            self.interface.borrow().log_agree(&agr);
            self.agrs.add(agr);
        }

        // OK, done
        res
    }
}
impl JAAgreements for GlobalAgreementsDictator {
    type Message = Message;
    type Error = AgreementsDictatorError;

    #[inline]
    fn agree(&mut self, agr: Agreement<Self::Message>) -> Result<(), Self::Error> {
        // Do not advance if we're not the dictator
        if self.dictator == "<system>" {
            self.agrs.add(agr);
            Ok(())
        } else {
            Err(AgreementsDictatorError::NotTheDictator { id: agr.id().into(), agent: "<system>".into(), dictator: self.dictator.clone() })
        }
    }

    #[inline]
    fn agreed<'s>(&'s self) -> LocalSet<&'s Agreement<Self::Message>> { self.agrs.iter().collect() }
}

/// Provides agents with a global view on the agreed upon agreements.
///
/// This variation synchronizes new agreements if and only if it's a particular agent claiming it.
#[derive(Debug)]
pub struct AgreementsDictator<'v> {
    /// This agent
    agent:    &'v str,
    /// The only agent allowed to make changes.
    dictator: &'v str,

    /// The statements that this agent knows of.
    agrs: &'v LocalSet<Agreement<Message>>,
    /// A queue of statements that this agent pushed.
    pub(crate) queue: Vec<Agreement<Message>>,
}
impl<'v> JAAgreements for AgreementsDictator<'v> {
    type Message = Message;
    type Error = AgreementsDictatorError;

    #[inline]
    fn agree(&mut self, agr: Agreement<Self::Message>) -> Result<(), Self::Error> {
        // Do not advance if we're not the dictator
        if self.agent == self.dictator {
            self.queue.push(agr);
            Ok(())
        } else {
            Err(AgreementsDictatorError::NotTheDictator { id: agr.id().into(), agent: self.agent.into(), dictator: self.dictator.into() })
        }
    }

    #[inline]
    fn agreed<'s>(&'s self) -> LocalSet<&'s Agreement<Self::Message>> { self.agrs.iter().chain(self.queue.iter()).collect() }
}
impl<'a, 'v> JAAgreements for &'a mut AgreementsDictator<'v> {
    type Message = <AgreementsDictator<'v> as JAAgreements>::Message;
    type Error = <AgreementsDictator<'v> as JAAgreements>::Error;

    #[inline]
    fn agree(&mut self, agr: Agreement<Self::Message>) -> Result<(), Self::Error> { AgreementsDictator::agree(self, agr) }

    #[inline]
    fn agreed<'s>(&'s self) -> LocalSet<&'s Agreement<Self::Message>> { AgreementsDictator::agreed(self) }
}
