//  AGENT.rs
//    by Tim Müller
//
//  Description:
//!   Defines a little interface to conveniently build agent scripts.
//

use std::error;
use std::task::Poll;

use ::justact::collections::set::Set as _;
use ::justact::policies::{Extractor as _, Policy as _};
use slick::{GroundAtom, Program};
use thiserror::Error;

#[cfg(feature = "dataplane")]
use crate::dataplane::ScopedStoreHandle;
use crate::io::TracingView;
use crate::policy::slick::Extractor;

mod justact {
    pub use ::justact::actions::ConstructableAction;
    pub use ::justact::actors::{Agent, Synchronizer, View};
    pub use ::justact::auxillary::Identifiable;
    pub use ::justact::collections::set::{Set, SetAsync, SetSync};
    pub use ::justact::collections::{Recipient, Singleton};
    pub use ::justact::messages::ConstructableMessage;
}


/***** ERRORS *****/
/// Maps any error to a generic one.
pub fn cast(err: impl 'static + Send + error::Error) -> Box<dyn 'static + Send + error::Error> { Box::new(err) }



/// Defines errors for [`Agent`]s.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to extract the Slick policy from the given message")]
    Extract(#[source] crate::policy::slick::SyntaxError),
    #[error("Failed to interact with the [`View::agreed`]-set.")]
    SetAgreed(#[source] Box<dyn 'static + Send + error::Error>),
    #[error("Failed to interact with the [`View::stated`]-set.")]
    SetStated(#[source] Box<dyn 'static + Send + error::Error>),
    #[cfg(feature = "dataplane")]
    #[error("Failed to interact with the store")]
    Store(#[source] crate::dataplane::Error),
}





/***** HELPERS *****/
/// Represents a single script step.
#[derive(Debug)]
enum Step {
    /// [`AgentProgrammer::agree()`]
    Agree { msg: Program },
    /// [`AgentProgrammer::state()`]
    State { to: justact::Recipient<String>, msg: Program },
    /// [`AgentProgrammer::enact_on_truth()`], [`AgentProgrammer::enact_on_truths()`]
    EnactOnTruths { truths: Vec<GroundAtom> },
    /// [`AgentProgrammer::wait_for_truth()`], [`AgentProgrammer::wait_for_truths()`]
    WaitForTruths { truths: Vec<GroundAtom> },
    /// [`AgentProgrammer::wait_for_datum()`], [`AgentProgrammer::wait_for_data()`]
    #[cfg(feature = "dataplane")]
    WaitForData { data: Vec<((String, String), String)> },
    /// [`AgentProgrammer::read()`]
    #[cfg(feature = "dataplane")]
    Read { target: ((String, String), String), context: String },
    /// [`AgentProgrammer::write()`]
    #[cfg(feature = "dataplane")]
    Write { target: ((String, String), String), context: String, content: Vec<u8> },
}





/***** AUXILLARY *****/
/// Builder-like interface for an [`Agent`].
///
/// This exists to be able to add steps to the agent while being efficient around the ordering of
/// the list of steps.
pub struct AgentProgrammer<'a>(&'a mut Vec<Step>);
impl<'a> Drop for AgentProgrammer<'a> {
    #[inline]
    fn drop(&mut self) {
        // Reverse back before the agent can use 'em
        self.0.reverse();
    }
}

// Steps
impl<'a> AgentProgrammer<'a> {
    /// States a message as an agreement immediately once this step is reached.
    ///
    /// This is a Synchronizer-only action.
    ///
    /// # Arguments
    /// - `msg`: The message to state as an agreement.
    #[inline]
    pub fn agree(&mut self, msg: Program) -> &mut Self {
        self.0.push(Step::Agree { msg });
        self
    }



    /// States a message immediately once this step is reached.
    ///
    /// # Arguments
    /// - `to`: The [`Recipient`] encoding who to state to.
    /// - `msg`: The message to state.
    #[inline]
    pub fn state(&mut self, to: justact::Recipient<String>, msg: Program) -> &mut Self {
        self.0.push(Step::State { to, msg });
        self
    }

    /// States a message once a certain truth is in the agent's view.
    ///
    /// # Arguments
    /// - `truth`: The truth to watch for.
    /// - `to`: The [`Recipient`] encoding who to state to.
    /// - `msg`: The message to state once it has become available.
    #[inline]
    pub fn state_on_truth(&mut self, truth: GroundAtom, to: justact::Recipient<String>, msg: Program) -> &mut Self {
        self.0.push(Step::WaitForTruths { truths: Vec::from([truth]) });
        self.0.push(Step::State { to, msg });
        self
    }

    /// States a message once zero or more certain truths are in the agent's view.
    ///
    /// # Arguments
    /// - `truths`: The truths to watch for. This step will only trigger if all of them are
    ///   present.
    /// - `to`: The [`Recipient`] encoding who to state to.
    /// - `msg`: The message to state once it has become available.
    #[inline]
    pub fn state_on_truths(&mut self, truths: impl IntoIterator<Item = GroundAtom>, to: justact::Recipient<String>, msg: Program) -> &mut Self {
        self.0.push(Step::WaitForTruths { truths: truths.into_iter().collect() });
        self.0.push(Step::State { to, msg });
        self
    }



    /// Enacts an action once a certain truth is in the agent's view.
    ///
    /// Unlike [`AgentProgrammer::state_on_truth()`], most of the action is derived. Specifically,
    /// we assume this agent is the actor; its payload are all messages which contain one of the
    /// waited-for truths; and the basis is automatically the first of those that is agreed.
    ///
    /// # Arguments
    /// - `truth`: The truth to watch for.
    #[inline]
    pub fn enact_on_truth(&mut self, truth: GroundAtom) -> &mut Self {
        self.0.push(Step::EnactOnTruths { truths: Vec::from([truth]) });
        self
    }

    /// Enacts an action once zero or more certain truths are in the agent's view.
    ///
    /// Unlike [`AgentProgrammer::state_on_truths()`], most of the action is derived. Specifically,
    /// we assume this agent is the actor; its payload are all messages which contain one of the
    /// waited-for truths; and the basis is automatically the first of those that is agreed.
    ///
    /// # Arguments
    /// - `truths`: The truths to watch for. This step will only trigger if all of them are
    ///   present.
    #[inline]
    pub fn enact_on_truths(&mut self, truths: impl IntoIterator<Item = GroundAtom>) -> &mut Self {
        self.0.push(Step::EnactOnTruths { truths: truths.into_iter().collect() });
        self
    }



    /// Wait for some truth to become available.
    ///
    /// # Arguments
    /// - `truth`: The truth to watch for.
    #[inline]
    pub fn wait_for_truth(&mut self, truth: GroundAtom) -> &mut Self {
        self.0.push(Step::WaitForTruths { truths: Vec::from([truth]) });
        self
    }

    /// Wait for some truths to become available.
    ///
    /// # Arguments
    /// - `truths`: The truths to watch for.
    #[inline]
    pub fn wait_for_truths(&mut self, truths: impl IntoIterator<Item = GroundAtom>) -> &mut Self {
        self.0.push(Step::WaitForTruths { truths: truths.into_iter().collect() });
        self
    }

    /// Wait for a dataset to become available.
    ///
    /// # Arguments
    /// - `data`: The data identifier to watch for.
    #[cfg(feature = "dataplane")]
    #[inline]
    pub fn wait_for_datum(&mut self, ((data_auth, data_id), data_func): ((impl Into<String>, impl Into<String>), impl Into<String>)) -> &mut Self {
        self.0.push(Step::WaitForData { data: vec![((data_auth.into(), data_id.into()), data_func.into())] });
        self
    }

    /// Wait for some datasets to become available.
    ///
    /// # Arguments
    /// - `data`: The list of data identifiers to watch for.
    #[cfg(feature = "dataplane")]
    #[inline]
    pub fn wait_for_data<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        &mut self,
        data: impl IntoIterator<Item = ((S1, S2), S3)>,
    ) -> &mut Self {
        self.0.push(Step::WaitForData {
            data: data
                .into_iter()
                .map(|((target_auth, target_id), target_func)| ((target_auth.into(), target_id.into()), target_func.into()))
                .collect(),
        });
        self
    }



    /// Read something from a dataset.
    ///
    /// # Arguments
    /// - `target`: The name of the data to write to.
    /// - `context`: The ID of the action that justifies this.
    #[cfg(feature = "dataplane")]
    #[inline]
    pub fn read(
        &mut self,
        ((target_auth, target_id), target_func): ((impl Into<String>, impl Into<String>), impl Into<String>),
        context: impl Into<String>,
    ) -> &mut Self {
        self.0.push(Step::Read { target: ((target_auth.into(), target_id.into()), target_func.into()), context: context.into() });
        self
    }

    /// Write something to a dataset.
    ///
    /// # Arguments
    /// - `target`: The name of the data to write to.
    /// - `context`: The ID of the action that justifies this.
    /// - `content`: Something to write to the dataset.
    #[cfg(feature = "dataplane")]
    #[inline]
    pub fn write(
        &mut self,
        ((target_auth, target_id), target_func): ((impl Into<String>, impl Into<String>), impl Into<String>),
        context: impl Into<String>,
        content: impl Into<Vec<u8>>,
    ) -> &mut Self {
        self.0.push(Step::Write {
            target:  ((target_auth.into(), target_id.into()), target_func.into()),
            context: context.into(),
            content: content.into(),
        });
        self
    }
}





/***** LIBRARY *****/
/// Defines a script for an agent.
pub struct Agent {
    /// The ID of this agent.
    id:    String,
    /// Defines listeners at each state (every place is one).
    ///
    /// Note, reversed for efficient popping!
    steps: Vec<Step>,
    /// Defines the store, if any, to listen for events there.
    #[cfg(feature = "dataplane")]
    store: Option<ScopedStoreHandle>,
}

// Constructors
impl Agent {
    /// Builds a new Agent.
    ///
    /// If you have a dataplane enabled, this will build without it. See [`Agent::with_store()`]
    /// instead.
    ///
    /// # Arguments
    /// - `id`: The name of this agent.
    ///
    /// # Returns
    /// A new Agent that can be programmed with steps.
    #[inline]
    pub const fn new(id: String) -> Self {
        Self {
            id,
            steps: Vec::new(),
            #[cfg(feature = "dataplane")]
            store: None,
        }
    }

    /// Builds a new script with the given dataplane store to listen for data events.
    ///
    /// If you don't have a dataplane, see [`Agent::new()`] instead.
    ///
    /// # Arguments
    /// - `id`: The name of this agent.
    /// - `store`: The [`ScopedStoreHandle`] representing the relevant agent's access to the
    ///   dataplane.
    ///
    /// # Returns
    /// A new Agent that can be programmed with steps.
    #[cfg(feature = "dataplane")]
    #[inline]
    pub const fn with_store(id: String, store: ScopedStoreHandle) -> Self { Self { id, steps: Vec::new(), store: Some(store) } }



    /// Returns an interface to add new steps to the agent.
    ///
    /// # Returns
    /// An [`AgentProgrammer`] interface that can add new steps.
    #[inline]
    pub fn program(&mut self) -> AgentProgrammer<'_> {
        // Reverse the list initially, just in case it already contains values
        self.steps.reverse();
        AgentProgrammer(&mut self.steps)
    }
}

// Step processing
impl Agent {
    /// Processes a single step as if this Agent is an [`Agent`](justact::Agent).
    ///
    /// Hence, synchronizers can call this to handle everything except synchronization.
    ///
    /// # Arguments
    /// - `view`: The [`View`] to interact with the world with.
    ///
    /// # Returns
    /// A [`Poll`] encoding whether to continue or whether this agent is dead.
    ///
    /// # Errors
    /// This function can error, for sure. Jep.
    fn process_step<A, S, E, SM, SA>(&mut self, mut view: TracingView<A, S, E>) -> Result<Poll<()>, Error>
    where
        A: justact::Set<SM>,
        S: justact::SetAsync<str, SM>,
        E: justact::SetAsync<str, SA>,
        SM: justact::ConstructableMessage<AuthorId = str, Payload = Program>,
        SA: justact::ConstructableAction<ActorId = str, Message = SM>,
    {
        let Some(step) = self.steps.last() else { return Ok(Poll::Ready(())) };
        match step {
            Step::State { to: _, msg: _ } => {
                let Step::State { to, msg } = self.steps.pop().unwrap() else { unreachable!() };
                let msg = SM::new(self.id.clone(), msg.clone());
                view.state(msg.clone()).map_err(cast).map_err(Error::SetStated)?;
                view.gossip(to, msg).map_err(cast).map_err(Error::SetStated)?;

                // We still might need a next step, though
                if self.steps.is_empty() { Ok(Poll::Ready(())) } else { Ok(Poll::Pending) }
            },

            Step::EnactOnTruths { truths } => {
                let mut truths = truths.clone();

                // Ensure there is at least one basis
                let basis = view.0.agreed.iter().map_err(cast).map_err(Error::SetAgreed)?.next();
                if basis.is_none() {
                    return Ok(Poll::Pending);
                }

                // For all the statements in the view...
                let mut msgs: Vec<SM> = Vec::new();
                for stmt in view.0.stated.iter().map_err(cast).map_err(Error::SetStated)? {
                    // ...extract the truths from this message...
                    let pol = match Extractor.extract(&justact::Singleton(stmt)) {
                        Ok(pol) => pol,
                        Err(err) => return Err(Error::Extract(err)),
                    };
                    let denot = pol.truths();

                    // ...and then check if they are all contained
                    let mut to_remove = Vec::new();
                    for (i, truth) in <[_]>::iter(&truths).enumerate() {
                        if denot.contains(truth).unwrap() {
                            // Remove it from the truths
                            to_remove.push(i);
                        }
                    }
                    if !to_remove.is_empty() {
                        msgs.push(stmt.clone());
                        truths = truths
                            .into_iter()
                            .enumerate()
                            .filter_map(|(i, truth)| if !<[_]>::contains(&to_remove, &i) { Some(truth) } else { None })
                            .collect();
                    }
                    if truths.is_empty() {
                        break;
                    }
                }

                // If not all truths are found, we need to wait
                if !truths.is_empty() {
                    return Ok(Poll::Pending);
                }

                // Now build the action and enact it!
                self.steps.pop();
                view.enact(SA::new(self.id.clone(), basis.unwrap().clone(), msgs.into_iter().collect())).map_err(cast).map_err(Error::SetStated)?;

                // We still might need a next step, though
                if self.steps.is_empty() { Ok(Poll::Ready(())) } else { Ok(Poll::Pending) }
            },

            Step::WaitForTruths { truths } => {
                let mut truths = truths.clone();

                // For all the statements in the view...
                for stmt in
                    view.0.agreed.iter().map_err(cast).map_err(Error::SetAgreed)?.chain(view.0.stated.iter().map_err(cast).map_err(Error::SetStated)?)
                {
                    // ...extract the truths from this message...
                    let pol = match Extractor.extract(&justact::Singleton(stmt)) {
                        Ok(pol) => pol,
                        Err(err) => return Err(Error::Extract(err)),
                    };
                    let denot = pol.truths();

                    // ...and then check if they are all contained
                    let mut to_remove = Vec::new();
                    for (i, truth) in <[_]>::iter(&truths).enumerate() {
                        if denot.contains(truth).unwrap() {
                            // Remove it from the truths
                            to_remove.push(i);
                        }
                    }
                    truths = truths
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, truth)| if !<[_]>::contains(&to_remove, &i) { Some(truth) } else { None })
                        .collect();
                    if truths.is_empty() {
                        break;
                    }
                }

                // If not all truths are found, we need to wait
                if truths.is_empty() {
                    self.steps.pop();
                    if self.steps.is_empty() {
                        return Ok(Poll::Ready(()));
                    }
                }
                Ok(Poll::Pending)
            },

            #[cfg(feature = "dataplane")]
            Step::WaitForData { data } => {
                // Cross 'em out
                for datum in data {
                    if !self.store.as_ref().expect("Cannot wait for data without a store!").exists(datum) {
                        return Ok(Poll::Pending);
                    }
                }
                self.steps.pop();
                if self.steps.is_empty() { Ok(Poll::Ready(())) } else { Ok(Poll::Pending) }
            },

            #[cfg(feature = "dataplane")]
            Step::Read { target: _, context: _ } => {
                let Step::Read { target, context } = self.steps.pop().unwrap() else { unreachable!() };
                if self
                    .store
                    .as_ref()
                    .expect("Cannot read without a store!")
                    .read(((&target.0.0, &target.0.1), &target.1), context)
                    .map_err(Error::Store)?
                    .is_none()
                {
                    panic!("Cannot read from non-existing dataset (({:?}, {:?}), {:?})", target.0.0, target.0.1, target.1);
                }
                if self.steps.is_empty() { Ok(Poll::Ready(())) } else { Ok(Poll::Pending) }
            },
            #[cfg(feature = "dataplane")]
            Step::Write { target: _, context: _, content: _ } => {
                let Step::Write { target, context, content } = self.steps.pop().unwrap() else { unreachable!() };
                self.store.as_ref().expect("Cannot write without a store!").write(target, context, content).map_err(Error::Store)?;
                if self.steps.is_empty() { Ok(Poll::Ready(())) } else { Ok(Poll::Pending) }
            },

            // Synchronizer-only steps
            Step::Agree { .. } => panic!("Cannot handle Synchronizer step in agent"),
        }
    }
}

// JustAct
impl justact::Identifiable for Agent {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { &self.id }
}
impl justact::Agent<Program> for Agent {
    type Error = Error;

    fn poll<A, S, E, SM, SA>(&mut self, mut view: justact::View<Self::Id, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        A: justact::Set<SM>,
        S: justact::SetAsync<Self::Id, SM>,
        E: justact::SetAsync<Self::Id, SA>,
        SM: justact::ConstructableMessage<AuthorId = Self::Id, Payload = Program>,
        SA: justact::ConstructableAction<ActorId = Self::Id, Message = SM>,
    {
        // Process the current step, if any
        self.process_step(TracingView(&mut view))
    }
}
impl justact::Synchronizer<Program> for Agent {
    type Error = Error;

    fn poll<A, S, E, SM, SA>(&mut self, mut view: justact::View<Self::Id, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        A: justact::SetSync<SM>,
        S: justact::SetAsync<Self::Id, SM>,
        E: justact::SetAsync<Self::Id, SA>,
        SM: justact::ConstructableMessage<AuthorId = Self::Id, Payload = Program>,
        SA: justact::ConstructableAction<ActorId = Self::Id, Message = SM>,
    {
        let mut view = TracingView(&mut view);

        // Catch any step that is for us
        let Some(step) = self.steps.last() else { return Ok(Poll::Ready(())) };
        match step {
            Step::Agree { msg: _ } => {
                // Publish the agreement
                let Step::Agree { msg } = self.steps.pop().unwrap() else { unreachable!() };
                view.agree([SM::new(self.id.clone(), msg)]).map_err(cast).map_err(Error::SetAgreed)?;

                // Done
                if self.steps.is_empty() { Ok(Poll::Ready(())) } else { Ok(Poll::Pending) }
            },

            // The rest is up to the default step processing
            _ => self.process_step(view),
        }
    }
}
