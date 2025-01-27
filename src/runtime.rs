//  RUNTIME.rs
//    by Lut99
//
//  Created:
//    13 Jan 2025, 15:05:42
//  Last edited:
//    24 Jan 2025, 22:41:00
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the main runtime regarding the JustAct policy engine.
//

use std::error;
use std::ops::ControlFlow;
use std::task::Poll;

#[cfg(feature = "log")]
use log::debug;
use thiserror::Error;

use crate::io::TracingSet;
use crate::sets::{Actions, Agreements, Statements, Times};

mod justact {
    pub use ::justact::actors::{Agent, Synchronizer, View};
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
    times:   TracingSet<Times>,
    /// Defines the set of all agreements.
    agreed:  TracingSet<Agreements>,
    /// Defines the set of all stated messages.
    stated:  TracingSet<Statements>,
    /// Defines the set of all enacted actions.
    enacted: TracingSet<Actions>,
}
impl Default for Runtime {
    #[inline]
    fn default() -> Self { Self::new() }
}
impl Runtime {
    /// Constructor for the Runtime that initializes it with nothing done yet.
    ///
    /// # Returns
    /// An empty Runtime, ready to [run](Runtime::run()).
    #[inline]
    pub fn new() -> Self {
        Self {
            times:   TracingSet(Times::new()),
            agreed:  TracingSet(Agreements::new()),
            stated:  TracingSet(Statements::new()),
            enacted: TracingSet(Actions::new()),
        }
    }
}
impl justact::Runtime for Runtime {
    type MessageId = (String, u32);
    type ActionId = (String, char);
    type AgentId = str;
    type SynchronizerId = str;
    type Payload = str;
    type Timestamp = u64;
    type Error = Error;


    #[inline]
    fn run<A>(
        &mut self,
        agents: impl IntoIterator<Item = A>,
        mut synchronizer: impl justact::Synchronizer<Self::MessageId, Self::ActionId, Self::Payload, Self::Timestamp, Id = Self::SynchronizerId>,
    ) -> Result<(), Self::Error>
    where
        A: justact::Agent<Self::MessageId, Self::ActionId, Self::Payload, Self::Timestamp, Id = Self::AgentId>,
    {
        // First, register any non-registered agents
        self.stated.register(synchronizer.id());
        self.enacted.register(synchronizer.id());
        let mut agents: Vec<A> = agents.into_iter().collect();
        for agent in &agents {
            self.stated.register(agent.id());
            self.enacted.register(agent.id());
        }

        // Enter a loop to execute agents
        loop {
            // Go through the agents and keep the ones that want to be kept
            agents = agents
                .into_iter()
                .filter_map(|mut agent| {
                    // Run the agent
                    let agent_id: String = agent.id().into();
                    match agent.poll(justact::View {
                        times:   &self.times,
                        agreed:  &self.agreed,
                        stated:  self.stated.scope(&agent_id),
                        enacted: self.enacted.scope(&agent_id),
                    }) {
                        Ok(Poll::Ready(_)) => {
                            #[cfg(feature = "log")]
                            debug!("Agent {agent_id:?} is complete.");
                            None
                        },
                        Ok(Poll::Pending) => Some(Ok(agent)),
                        Err(err) => Some(Err(Error::Agent { id: agent_id, err: Box::new(err) })),
                    }
                })
                .collect::<Result<Vec<A>, Error>>()?;

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
