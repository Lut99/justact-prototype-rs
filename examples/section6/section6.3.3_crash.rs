//  SECTION 6.3.3 - CRASH.rs
//    by Lut99
//
//  Created:
//    30 Jan 2025, 18:28:07
//  Last edited:
//    31 Jan 2025, 18:15:40
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the third example from the paper.
//

mod agents;
mod trace;

use agents::{Agent, Amy, Bob, Consortium, Dan, Script, StAntonius, Surf};
use clap::Parser;
use error_trace::toplevel;
use humanlog::{DebugMode, HumanLogger};
use justact::runtime::Runtime as _;
use justact_prototype::dataplane::StoreHandle;
use justact_prototype::runtime::System;
use log::{debug, error, info};


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

    /// Where to output the trace to. Use '-' to output to stdout.
    #[clap(short, long, default_value = "-")]
    output: String,
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
    if args.output == "-" {
        debug!("Registering stdout event handler");
        justact_prototype::io::register_event_handler(trace::StdoutEventHandler);
    } else {
        debug!("Registering file event handler to {:?}", args.output);
        match trace::FileEventHandler::new(&args.output) {
            Ok(handler) => justact_prototype::io::register_event_handler(handler),
            Err(err) => {
                error!("{}", toplevel!(("Failed to create file {:?} to write events to", args.output), err));
                std::process::exit(1);
            },
        }
    }

    // Create the agents
    let dataplane = StoreHandle::new();
    let agents: [Agent; 5] = [
        Amy::new(Script::Section6_3_3_crash, &dataplane).into(),
        Bob::new(Script::Section6_3_3_crash, &dataplane).into(),
        Dan::new(Script::Section6_3_3_crash).into(),
        StAntonius::new(Script::Section6_3_3_crash, &dataplane).into(),
        Surf::new(Script::Section6_3_3_crash, &dataplane).into(),
    ];
    let sync = Consortium::new(Script::Section6_3_3_crash);

    // Run the runtime!
    let mut runtime = System::new();
    if let Err(err) = runtime.run::<Agent>(agents, sync) {
        error!("{}", toplevel!(("Failed to run runtime"), err));
        std::process::exit(1);
    }

    // Done!
}
