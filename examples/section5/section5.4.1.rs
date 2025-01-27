//  SECTION 5.4.1.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:49:57
//  Last edited:
//    22 Jan 2025, 17:02:01
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the first example from section 5.4 of the JustAct paper
//!   \[1\].
//

mod agents;
mod error;
mod trace;

use agents::{Agent, Amdex, Amy, Consortium, Dan, Script, StAntonius, Surf};
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
    justact_prototype::io::register_trace_handler(trace::StdoutTraceHandler);

    // Create the agents
    let dataplane = StoreHandle::new();
    let agents: [Agent; 5] = [
        Amdex::new(Script::Section5_4_1, &dataplane).into(),
        Amy::new(Script::Section5_4_1, &dataplane).into(),
        Dan::new(Script::Section5_4_1, &dataplane).into(),
        StAntonius::new(Script::Section5_4_1, &dataplane).into(),
        Surf::new(Script::Section5_4_1, &dataplane).into(),
    ];
    let sync = Consortium::new(Script::Section5_4_1, &dataplane);

    // Run the runtime!
    let mut runtime = Runtime::new();
    if let Err(err) = runtime.run::<Agent>(agents, sync) {
        error!("{}", trace!(("Failed to run runtime"), err));
        std::process::exit(1);
    }

    // Done!
}
