//  PAPER.rs
//    by Lut99
//
//  Created:
//    16 Apr 2024, 11:00:44
//  Last edited:
//    26 Nov 2024, 12:06:21
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the examples from the paper using $Datalog^\neg$ in the
//!   simple simulation environment.
//!   
//!   Contains the full Step 1-example.
//

// Modules
mod paper;

// Imports
use clap::Parser;
use console::Style;
use error_trace::trace;
use humanlog::{DebugMode, HumanLogger};
use justact_prototype::policy::datalog::Extractor;
use justact_prototype::Simulation;
use log::{error, info};

use crate::paper::{AbstractAgent, Administrator, Amy, Anton, Consortium};


/***** ARGUMENTS *****/
/// Defines arguments for this example.
#[derive(Debug, Parser)]
struct Arguments {
    /// If given, enables INFO- and DEBUG-level logging.
    #[clap(long, global = true)]
    debug: bool,
    /// If given, enables INFO-, DEBUG- and TRACE-level logging. Implies '--debug'.
    #[clap(long, global = true)]
    trace: bool,
}





/***** ENTRYPOINT *****/
fn main() {
    // Read CLI args
    let args = Arguments::parse();

    // Setup logger
    if let Err(err) = HumanLogger::terminal(DebugMode::from_flags(args.trace, args.debug)).init() {
        eprintln!("WARNING: Failed to setup logger: {err} (no logging for this session)");
    }
    info!("{} - v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));

    // Build the Simulation
    let mut sim: Simulation<AbstractAgent> = Simulation::with_capacity("consortium", 1);
    sim.register(Consortium, Style::new().bold().cyan());
    sim.register(Administrator, Style::new().bold().yellow());
    sim.register(Amy, Style::new().bold().green());
    sim.register(Anton, Style::new().bold().magenta());

    // Run it
    println!();
    if let Err(err) = sim.run::<Extractor>() {
        error!("{}", trace!(("Failed to run simulation"), err));
        std::process::exit(1);
    };

    // Done!
    println!();
    println!("Done.");
    println!();
}
