//  SIMULATION.rs
//    by Lut99
//
//  Created:
//    16 Apr 2024, 11:06:51
//  Last edited:
//    26 Nov 2024, 11:51:31
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the main simulation loop that can run agents.
//

use std::any::type_name;
use std::cell::RefCell;
use std::collections::HashSet;
use std::convert::Infallible;
use std::error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::rc::Rc;

use console::Style;
use justact::agents::{AgentPoll, RationalAgent};
use justact::auxillary::Identifiable;
use justact::policy::Extractor;
use justact::set::LocalSet;
use log::{debug, info};
use stackvec::StackVec;

use crate::agreements::GlobalAgreementsDictator;
use crate::interface::Interface;
use crate::statements::{GlobalStatements, Message, Target};
use crate::times::GlobalTimesDictator;


/***** ERROR *****/
/// Defines errors originating in the [`Simulation`].
#[derive(Debug)]
pub enum Error<E> {
    /// Some agent errored.
    AgentPoll { agent: String, err: E },
}
impl<E: Display> Display for Error<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use Error::*;
        match self {
            AgentPoll { agent, .. } => write!(f, "Failed to poll agent {agent}"),
        }
    }
}
impl<E: 'static + error::Error> error::Error for Error<E> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use Error::*;
        match self {
            AgentPoll { err, .. } => Some(err),
        }
    }
}





/***** LIBRARY *****/
/// Runs a simulation with the given agents.
///
/// The simulation runs until all given agents are dead.
///
/// # Generics
/// - `A`: Some generic kind over the specific [`Agent`] required for this implementation. It is recommended to make some sum Agent type yourself that abstracts over the different ones if necessary.
#[derive(Debug)]
pub struct Simulation<A> {
    /// The (alive!) agents in the simulation.
    agents:    Vec<A>,
    /// A set of action (identifiers) of the ones we've already audited
    audited:   HashSet<String>,
    /// An interface we use to log whatever happens in pretty ways.
    interface: Rc<RefCell<Interface>>,

    /// The globally synchronized agreements.
    agrs:  GlobalAgreementsDictator,
    /// The globally synchronized timestamps.
    times: GlobalTimesDictator,
    /// The local statements.
    stmts: GlobalStatements,
}
impl<A> Simulation<A> {
    /// Creates a new Simulation with no agents registered yet.
    ///
    /// # Arguments
    /// - `dictator`: The agent that gets to update all globally synchronized agreements.
    ///
    /// # Returns
    /// An empty simulation that wouldn't run anything.
    #[inline]
    pub fn new(dictator: impl AsRef<str>) -> Self {
        let dictator: &str = dictator.as_ref();
        info!("Creating demo Simulation<{}>", type_name::<A>());

        // Build an interface with ourselves registered
        let mut interface: Interface = Interface::new();
        interface.register("<system>", Style::new().bold());

        // Create ourselves with that
        let interface: Rc<RefCell<Interface>> = Rc::new(RefCell::new(interface));
        Self {
            agrs: GlobalAgreementsDictator::new(dictator, interface.clone()),
            times: GlobalTimesDictator::new(dictator, interface.clone()),
            stmts: GlobalStatements::new(interface.clone()),
            agents: Vec::new(),
            audited: HashSet::new(),
            interface,
        }
    }

    /// Creates a new Simulation with no agents registered yet, but space to do so before re-allocation is triggered.
    ///
    /// # Arguments
    /// - `dictator`: The agent that gets to update all globally synchronized agreements.
    /// - `capacity`: The number of agents for which there should _at least_ be space.
    ///
    /// # Returns
    /// An empty simulation that wouldn't run anything but that has space for at least `capacity` agents.
    #[inline]
    pub fn with_capacity(dictator: impl AsRef<str>, capacity: usize) -> Self {
        let dictator: &str = dictator.as_ref();
        info!("Creating demo Simulation<{}> (with capacity '{}')", type_name::<A>(), capacity);

        // Build an interface with ourselves registered
        let mut interface: Interface = Interface::new();
        interface.register("<system>", Style::new().bold());

        // Create ourselves with that
        let interface: Rc<RefCell<Interface>> = Rc::new(RefCell::new(interface));
        Self {
            agrs: GlobalAgreementsDictator::new(dictator, interface.clone()),
            times: GlobalTimesDictator::new(dictator, interface.clone()),
            stmts: GlobalStatements::new(interface.clone()),
            agents: Vec::with_capacity(capacity),
            audited: HashSet::new(),
            interface,
        }
    }

    /// Builds a new Simulation with the given set of agents registered to it from the get-go.
    ///
    /// # Arguments
    /// - `dictator`: The agent that gets to update all globally synchronized agreements.
    /// - `agents`: Some list of `A`gents that should be registered right away.
    ///
    /// # Returns
    /// A Simulation with the given `agents` already registered in it.
    #[inline]
    pub fn with_agents(dictator: impl AsRef<str>, agents: impl IntoIterator<Item = A>) -> Self {
        let dictator: &str = dictator.as_ref();
        info!("Creating demo Simulation<{}> with agents", type_name::<A>());

        // Build an interface with ourselves registered
        let mut interface: Interface = Interface::new();
        interface.register("<system>", Style::new().bold());

        // Create agents out of the given iterator, logging as we go
        let agents: Vec<A> = agents
            .into_iter()
            .enumerate()
            .map(|(i, a)| {
                debug!("Registered agent {}", i);
                a
            })
            .collect();

        // Now built self
        let interface: Rc<RefCell<Interface>> = Rc::new(RefCell::new(interface));
        Self {
            agrs: GlobalAgreementsDictator::new(dictator, interface.clone()),
            times: GlobalTimesDictator::new(dictator, interface.clone()),
            stmts: GlobalStatements::new(interface.clone()),
            agents,
            audited: HashSet::new(),
            interface,
        }
    }
}
impl<A: Identifiable<Id = str>> Simulation<A> {
    /// Registers a new agent after creation.
    ///
    /// # Arguments
    /// - `agent`: The new `A`gent to register.
    /// - `style`: A [`Style`] that is used to format the agent's ID during logging.
    #[inline]
    pub fn register(&mut self, agent: impl Into<A>, style: Style) {
        debug!("Registered agent {}", self.agents.len());

        // Register the agent in the statements
        let agent: A = agent.into();
        self.stmts.register(&agent);

        // Register the agent in the interface
        self.interface.borrow_mut().register(agent.id(), style);

        // Put it in the simulation
        self.agents.push(agent.into());
    }
}
impl<A> Simulation<A>
where
    A: Identifiable<Id = str>,
    A: RationalAgent<Message = Message, Target = Target, Error = Infallible>,
{
    /// Polls all the agents in the simulation once.
    ///
    /// # Returns
    /// True if at least one agent is alive, or false otherwise.
    ///
    /// # Errors
    /// This function errors if any of the agents fails to communicate with the end-user or other agents.
    pub fn poll(&mut self) -> Result<bool, Error<<A as RationalAgent>::Error>> {
        let Self { agents, agrs, times, stmts, .. } = self;
        info!("Starting new agent iteration");

        // Iterate over the agents and only keep those that report they wanna be kept
        let mut agent_next: StackVec<64, A> = StackVec::new();
        for (i, mut agent) in agents.drain(..).enumerate() {
            debug!("Polling agent {}...", i);

            // Prepare calling the agent's poll method
            let id: String = agent.id().into();
            agrs.scope(&id, |agrs| {
                times.scope(&id, |times| {
                    stmts.scope(&id, |stmts| {
                        // Do the call then
                        match agent.poll(agrs, times, stmts) {
                            Ok(AgentPoll::Alive) => Ok(agent_next.push(agent)),
                            Ok(AgentPoll::Dead) => Ok(()),
                            Err(err) => Err(Error::AgentPoll { agent: id.clone(), err }),
                        }
                    })
                })
            })?;
        }

        // Now re-instante those kept and return whether we're done
        self.agents.extend(agent_next);
        Ok(!self.agents.is_empty())
    }

    /// Runs the simulation until no more agents are alive.
    ///
    /// # Errors
    /// This function errors if any of the agents fails to communicate with the end-user or other agents.
    #[inline]
    pub fn run<E>(&mut self) -> Result<(), Error<<A as RationalAgent>::Error>>
    where
        E: for<'e> Extractor<&'e Message>,
    {
        loop {
            // Run the next iteration
            let reiterate: bool = self.poll()?;

            // Run an audit
            debug!("Running audit on {} actions...", self.stmts.encts.values().map(LocalSet::len).sum::<usize>());
            for enct in self.stmts.encts.values().flat_map(LocalSet::iter) {
                // Audit if we haven't yet
                if !self.audited.contains(enct.id()) {
                    if let Err(expl) = enct.audit::<E, GlobalStatements, GlobalAgreementsDictator>(&self.stmts, &self.agrs) {
                        // Write the problem
                        self.interface.borrow().error_audit("<system>", enct, expl);
                    }
                    self.audited.insert(enct.id().into());
                }
            }

            // Stop if no agents are alive
            if !reiterate {
                return Ok(());
            }
        }
    }
}
