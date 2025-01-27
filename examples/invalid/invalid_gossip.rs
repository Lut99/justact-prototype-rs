//  INVALID GOSSIP.rs
//    by Lut99
//
//  Created:
//    26 Jan 2025, 17:40:50
//  Last edited:
//    26 Jan 2025, 18:02:33
//  Auto updated?
//    Yes
//
//  Description:
//!   Shows an example where an agent illegally states on behalf of
//!   another agent.
//

use std::convert::Infallible;
use std::error;
use std::ops::ControlFlow;
use std::task::Poll;

use clap::Parser;
use error_trace::trace;
use humanlog::{DebugMode, HumanLogger};
use justact::actions::ConstructableAction;
use justact::actors::{Agent, Synchronizer, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Selector;
use justact::collections::map::{Map, MapAsync, MapSync};
use justact::messages::ConstructableMessage;
use justact::runtime::Runtime as _;
use justact::times::{Times, TimesSync};
use justact_prototype::Runtime;
use justact_prototype::io::{Trace, TraceHandler};
use log::{error, info};


/***** HELPERS *****/
/// The behaviour of the [`Agent`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Behaviour {
    /// Honest gossiper.
    Honest,
    /// Malicious gossiper.
    Malicious,
}

/// A [`TraceHandler`] that writes to stdout.
pub struct StdoutTraceHandler;
impl TraceHandler for StdoutTraceHandler {
    #[inline]
    fn handle(&self, trace: Trace) -> Result<(), Box<dyn error::Error>> {
        println!("{}", serde_json::to_string(&trace).map_err(Box::new)?);
        Ok(())
    }
}





/***** AGENTS *****/
// Simple synchronizer agent that publishes a trivial agreement.
struct Environment;
impl Identifiable for Environment {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "environment" }
}
impl Synchronizer<(String, u32), (String, char), str, u64> for Environment {
    type Error = Infallible;

    #[inline]
    fn poll<T, A, S, E, SM, SA>(&mut self, _view: View<T, A, S, E>) -> Result<ControlFlow<()>, Self::Error>
    where
        T: TimesSync<Timestamp = u64>,
        A: MapSync<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, char), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        // We kill the system once its our turn, all agents did  their thing
        Ok(ControlFlow::Break(()))
    }
}

/// Simple agent that will either behave honestly (sending a message to someone else) or maliviously
/// (sending it illegally to the third).
struct Gossiper {
    /// Defines the ID of this agent.
    id: String,
    /// Defines the behaviour of this agent.
    behaviour: Behaviour,
}
impl Identifiable for Gossiper {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { &self.id }
}
impl Agent<(String, u32), (String, char), str, u64> for Gossiper {
    type Error = Infallible;

    #[inline]
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: Times<Timestamp = u64>,
        A: Map<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, char), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        match self.behaviour {
            Behaviour::Honest => match self.id.as_str() {
                // Amy sends her message to Bob
                "amy" => {
                    view.stated
                        .add(Selector::Agent("bob"), SM::new((String::new(), 1), self.id.clone(), "hello cho me lass how r u.".into()))
                        .unwrap();
                    Ok(Poll::Ready(()))
                },
                // Bob gossips Amy's message to Cho once he receives it
                "bob" => {
                    if let Some(msg) = view.stated.get(&("amy".into(), 1)).unwrap() {
                        if let Err(err) = view.stated.add(Selector::Agent("cho"), msg.clone()) {
                            error!("{}", trace!(("Bob failed to send Amy's message"), err));
                        }
                        Ok(Poll::Ready(()))
                    } else {
                        Ok(Poll::Pending)
                    }
                },
                // Cho awaits Amy's message
                "cho" => {
                    if view.stated.contains_key(&("amy".into(), 1)).unwrap() {
                        view.stated
                            .add(Selector::Agent("amy"), SM::new((String::new(), 1), self.id.clone(), "sup amy ye im good hby".into()))
                            .unwrap();
                        return Ok(Poll::Ready(()));
                    }
                    Ok(Poll::Pending)
                },
                _ => unreachable!(),
            },

            Behaviour::Malicious => {
                // Dan attempts to send Amy's message - but he never gets to see it!
                if let Err(err) = view.stated.add(Selector::All, SM::new((String::new(), 1), "amy".into(), "hello bob me lad how r u.".into())) {
                    error!("{}", trace!(("Dan failed to send Amy's message"), err));
                }
                Ok(Poll::Ready(()))
            },
        }
    }
}





/***** ARGUMENTS *****/
/// The binary's CLI arguments.
#[derive(Parser)]
struct Arguments {
    /// If given, enables additional INFO- and DEBUG-level statements.
    #[clap(long, global = true)]
    debug: bool,
    /// If given, enables additional TRACE-level statements. Implies `--debug`.
    #[clap(long, global = true)]
    trace: bool,
}





/***** ENTRYPOINT *****/
fn main() {
    // Parse args
    let args = Arguments::parse();

    // Setup the logger
    if let Err(err) = HumanLogger::terminal(if args.trace {
        DebugMode::Full
    } else if args.debug {
        DebugMode::Debug
    } else {
        DebugMode::HumanFriendly
    })
    .init()
    {
        eprintln!("WARNING: Failed to setup logger: {err} (no logging this session)");
    }
    info!("{} - v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));

    // Setup the trace callback
    justact_prototype::io::register_trace_handler(StdoutTraceHandler);

    // Create the agents
    let agents: [Gossiper; 4] = [
        Gossiper { id: "amy".into(), behaviour: Behaviour::Honest },
        Gossiper { id: "bob".into(), behaviour: Behaviour::Honest },
        Gossiper { id: "cho".into(), behaviour: Behaviour::Honest },
        Gossiper { id: "dan".into(), behaviour: Behaviour::Malicious },
    ];

    // Run the runtime!
    let mut runtime = Runtime::new();
    if let Err(err) = runtime.run::<Gossiper>(agents, Environment) {
        error!("{}", trace!(("Failed to run runtime"), err));
        std::process::exit(1);
    }

    // Done!
}
