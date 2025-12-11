//  INVALID ACTION.rs
//    by Lut99
//
//  Created:
//    29 Jan 2025, 23:05:56
//  Last edited:
//    31 Jan 2025, 18:18:17
//  Auto updated?
//    Yes
//
//  Description:
//!   An example where some agent, Dan, takes the time to make an illegal
//!   action in every way possible.
//

use std::convert::Infallible;
use std::error;
use std::task::Poll;

use clap::Parser;
use error_trace::toplevel;
use humanlog::{DebugMode, HumanLogger};
use justact::actions::ConstructableAction;
use justact::actors::{Agent, Synchronizer, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Recipient;
use justact::collections::map::{Map, MapAsync, MapSync};
use justact::collections::set::InfallibleSet as _;
use justact::messages::{ConstructableMessage, MessageSet};
use justact::runtime::Runtime as _;
use justact::times::{Times, TimesSync};
use justact_prototype::System;
use justact_prototype::auditing::Event;
use justact_prototype::io::EventHandler;
use log::{error, info};


/***** HELPERS *****/
/// An [`EventHandler`] that writes to stdout.
pub struct StdoutEventHandler;
impl EventHandler for StdoutEventHandler {
    #[inline]
    fn handle(&mut self, event: Event<str>) -> Result<(), Box<dyn 'static + Send + error::Error>> {
        println!("{}", serde_json::to_string(&event).map_err(|err| -> Box<dyn 'static + Send + error::Error> { Box::new(err) })?);
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
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: TimesSync<Timestamp = u64>,
        A: MapSync<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, char), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        // We publish a very simple agreement.
        let message = SM::new((String::new(), 1), self.id().into(), "error if illegal actions are taken.".into());
        view.stated.add(Recipient::All, message.clone()).unwrap();
        view.agreed.add(Agreement { message, at: 1 }).unwrap();
        view.times.add_current(1).unwrap();

        // And then an amendment. But note it's not a current one!
        let message = SM::new(
            (String::new(), 2),
            self.id().into(),
            "error if illegal actions are taken.\nerror if additional illegal actions are taken.".into(),
        );
        view.stated.add(Recipient::All, message.clone()).unwrap();
        view.agreed.add(Agreement { message, at: 2 }).unwrap();

        // Nothing to do for us
        Ok(Poll::Ready(()))
    }
}

/// Simple agent that will either behave honestly (sending a message to someone else) or maliviously
/// (sending it illegally to the third).
struct Dan;
impl Identifiable for Dan {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "dan" }
}
impl Agent<(String, u32), (String, char), str, u64> for Dan {
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
        // Wait until the agreements become available
        let (agree1, agree2): (&Agreement<SM, u64>, &Agreement<SM, u64>) =
            match (view.agreed.get(&(Environment.id().into(), 1)).unwrap(), view.agreed.get(&(Environment.id().into(), 2)).unwrap()) {
                (Some(agree1), Some(agree2)) => (agree1, agree2),
                _ => return Ok(Poll::Pending),
            };
        if !view.times.current().unwrap().contains(&1) {
            return Ok(Poll::Pending);
        }



        // First: Dan publishes an action which is not stated!!
        let message = SM::new((String::new(), 1), self.id().into(), "legal actions are taken.".into());
        view.enacted
            .add(
                Recipient::All,
                SA::new((String::new(), 'a'), self.id().into(), agree1.clone(), MessageSet::from_iter([agree1.message.clone(), message.clone()])),
            )
            .unwrap();

        // Oh no, Dan, be careful! He's stating an action which isn't based!!
        view.stated.add(Recipient::All, message.clone()).unwrap();
        view.enacted
            .add(Recipient::All, SA::new((String::new(), 'b'), self.id().into(), agree1.clone(), MessageSet::from_iter([message.clone()])))
            .unwrap();

        // Dan!! What's this!! Why would you publish an illegal action??
        let bad_msg = SM::new((String::new(), 2), self.id().into(), "illegal actions are taken.".into());
        view.stated.add(Recipient::All, bad_msg.clone()).unwrap();
        view.enacted
            .add(
                Recipient::All,
                SA::new((String::new(), 'c'), self.id().into(), agree1.clone(), MessageSet::from_iter([agree1.message.clone(), bad_msg])),
            )
            .unwrap();

        // I'm not believing my eyes... Dan, that's terrible... where is your heart, Dan? Why _not_ refer to a current agreement?
        view.enacted
            .add(
                Recipient::All,
                SA::new((String::new(), 'd'), self.id().into(), agree2.clone(), MessageSet::from_iter([agree2.message.clone(), message])),
            )
            .unwrap();



        // Dan is finally done with his crimes
        Ok(Poll::Ready(()))
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

    // Setup the event callback
    justact_prototype::io::register_event_handler(StdoutEventHandler);

    // Run the runtime!
    let mut runtime = System::new();
    if let Err(err) = runtime.run([Dan], Environment) {
        error!("{}", toplevel!(("Failed to run runtime"), err));
        std::process::exit(1);
    }

    // Done!
}
