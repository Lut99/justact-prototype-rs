//  RUNTIME.rs
//    by Lut99
//
//  Created:
//    13 Jan 2025, 15:05:42
//  Last edited:
//    29 Jan 2025, 22:06:25
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the main runtime regarding the JustAct policy engine.
//

use std::error;
use std::fmt::Debug;
use std::hash::Hash;
use std::task::Poll;

#[cfg(feature = "log")]
use log::debug;
use thiserror::Error;

use crate::policy::{PolicyReflect, PolicySerialize};
use crate::sets::{Actions, Agreements, Statements};

mod justact {
    pub use ::justact::actors::{Agent, Synchronizer, View};
    pub use ::justact::runtime::System;
}


/***** ERRORS *****/
/// Defines the errors emitted by the prototype [`Runtime`].
#[derive(Debug, Error)]
pub enum Error {
    #[error("Agent {id:?} failed.")]
    Agent {
        id:  String,
        #[source]
        err: Box<dyn 'static + Send + error::Error>,
    },
    #[error("Synchronizer {id:?} failed.")]
    Synchronizer {
        id:  String,
        #[source]
        err: Box<dyn 'static + Send + error::Error>,
    },
}





/***** LIBRARY *****/
/// Defines the prototype runtime that will do things in-memory.
pub struct System<P: ?Sized + ToOwned> {
    /// Defines the set of all agreements.
    agreed:  Agreements<P>,
    /// Defines the set of all stated messages.
    stated:  Statements<P>,
    /// Defines the set of all enacted actions.
    enacted: Actions<P>,
}
impl<P: ?Sized + ToOwned> Default for System<P> {
    #[inline]
    fn default() -> Self { Self::new() }
}
impl<P: ?Sized + ToOwned> System<P> {
    /// Constructor for the System that initializes it with nothing done yet.
    ///
    /// # Returns
    /// An empty System, ready to [run](Runtime::run()).
    #[inline]
    pub fn new() -> Self { Self { agreed: Agreements::new(), stated: Statements::new(), enacted: Actions::new() } }
}
impl<P: ?Sized + PolicyReflect + PolicySerialize + ToOwned> justact::System for System<P>
where
    P: 'static,
    P::Owned: 'static + Clone + Debug + Eq + Hash + Send + Sync,
{
    type AgentId = str;
    type SynchronizerId = str;
    type Payload = P;
    type Error = Error;


    #[inline]
    fn run<A>(
        &mut self,
        agents: impl IntoIterator<Item = A>,
        synchronizer: impl justact::Synchronizer<Self::Payload, Id = Self::SynchronizerId>,
    ) -> Result<(), Self::Error>
    where
        A: justact::Agent<Self::Payload, Id = Self::AgentId>,
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
        let mut synchronizer: Option<_> = Some(synchronizer);
        while synchronizer.is_some() || !agents.is_empty() {
            // Go through the agents and keep the ones that want to be kept
            agents = agents
                .into_iter()
                .filter_map(|mut agent| {
                    // Run the agent
                    let agent_id: String = agent.id().into();
                    match agent.poll(justact::View {
                        id:      agent_id.clone(),
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
            synchronizer = if let Some(mut sync) = synchronizer.take() {
                let sync_id: String = sync.id().into();
                match sync.poll(justact::View {
                    id:      sync_id.clone(),
                    agreed:  &mut self.agreed,
                    stated:  self.stated.scope(&sync_id),
                    enacted: self.enacted.scope(&sync_id),
                }) {
                    Ok(Poll::Ready(_)) => None,
                    Ok(Poll::Pending) => Some(sync),
                    Err(err) => return Err(Error::Synchronizer { id: sync_id, err: Box::new(err) }),
                }
            } else {
                None
            };
        }

        // OK, done!
        Ok(())
    }
}
