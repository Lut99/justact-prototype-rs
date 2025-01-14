//  RUNTIME.rs
//    by Lut99
//
//  Created:
//    13 Jan 2025, 15:05:42
//  Last edited:
//    14 Jan 2025, 16:42:36
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the main runtime regarding the JustAct policy engine.
//

use std::collections::HashMap;
use std::error;
use std::ops::ControlFlow;
use std::sync::Arc;

use thiserror::Error;

use crate::sets::{MapAsync, Times};
use crate::wire::{Action, Message};

mod justact {
    pub use ::justact::actors::{Agent, Synchronizer, View};
    pub use ::justact::agreements::Agreement;
    pub use ::justact::runtime::Runtime;
}


/***** ERRORS *****/
/// Defines the errors emitted by the prototype [`Runtime`].
#[derive(Debug, Error)]
pub enum Error {
    #[error("Agent {id:?} failed.")]
    Agent {
        id:  String,
        #[source]
        err: Box<dyn error::Error>,
    },
    #[error("Synchronizer {id:?} failed.")]
    Synchronizer {
        id:  String,
        #[source]
        err: Box<dyn error::Error>,
    },
}





/***** LIBRARY *****/
/// Defines the prototype runtime that will do things in-memory.
pub struct Runtime {
    /// Defines the set of all times (and which are current).
    times:   Times,
    /// Defines the set of all agreements.
    agreed:  HashMap<String, justact::Agreement<Arc<Message>, u128>>,
    /// Defines the set of all stated messages.
    stated:  MapAsync<Arc<Message>>,
    /// Defines the set of all enacted actions.
    enacted: MapAsync<Action>,
}
impl justact::Runtime for Runtime {
    type AgentId = str;
    type Error = Error;


    #[inline]
    fn run<A>(
        &mut self,
        agents: impl IntoIterator<Item = A>,
        mut synchronizer: impl justact::Synchronizer<Id = Self::AgentId>,
    ) -> Result<(), Self::Error>
    where
        A: justact::Agent<Id = Self::AgentId>,
    {
        // Enter a loop to execute agents
        let mut agents: Vec<A> = agents.into_iter().collect();
        loop {
            // Go through the agents
            for agent in &mut agents {
                // Run the agent
                let agent_id: String = agent.id().into();
                if let Err(err) = agent.poll(justact::View {
                    times:   &self.times,
                    agreed:  &self.agreed,
                    stated:  self.stated.scope(&agent_id),
                    enacted: self.enacted.scope(&agent_id),
                }) {
                    return Err(Error::Agent { id: agent_id, err: Box::new(err) });
                }
            }

            // Now run an update cycle through the synchronizer
            let sync_id: String = synchronizer.id().into();
            match synchronizer.poll(justact::View {
                times:   &mut self.times,
                agreed:  &mut self.agreed,
                stated:  self.stated.scope(&sync_id),
                enacted: self.enacted.scope(&sync_id),
            }) {
                Ok(ControlFlow::Continue(_)) => continue,
                Ok(ControlFlow::Break(_)) => break,
                Err(err) => return Err(Error::Synchronizer { id: sync_id, err: Box::new(err) }),
            }
        }

        // OK, done!
        Ok(())
    }
}
