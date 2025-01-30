//  SECTION 5.4.2.rs
//    by Lut99
//
//  Created:
//    22 Jan 2025, 16:57:21
//  Last edited:
//    30 Jan 2025, 20:44:56
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the second example from section 5.4 of the JustAct paper
//!   \[1\].
//

mod agents;
mod error;
mod trace;

use agents::{Agent, Bob, Consortium, Script, StAntonius, Surf};
use clap::Parser;
use error_trace::trace;
use humanlog::{DebugMode, HumanLogger};
use justact::runtime::Runtime as _;
use justact_prototype::dataplane::StoreHandle;
use justact_prototype::runtime::Runtime;
use log::{error, info};


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
    justact_prototype::io::register_event_handler(trace::StdoutEventHandler);

    // Create the agents
    let dataplane = StoreHandle::new();
    let agents: [Agent; 3] = [
        Bob::new(Script::Section5_4_2, &dataplane).into(),
        StAntonius::new(Script::Section5_4_2, &dataplane).into(),
        Surf::new(Script::Section5_4_2, &dataplane).into(),
    ];
    let sync = Consortium::new(Script::Section5_4_2);

    // Run the runtime!
    let mut runtime = Runtime::new();
    if let Err(err) = runtime.run::<Agent>(agents, sync) {
        error!("{}", trace!(("Failed to run runtime"), err));
        std::process::exit(1);
    }

    // Done!
}
