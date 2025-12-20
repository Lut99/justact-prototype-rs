//  SECTION 6.3.5.rs
//    by Lut99
//
//  Created:
//    22 Jan 2025, 16:57:21
//  Last edited:
//    31 Jan 2025, 18:15:46
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the fifth example from section 5.4 of the JustAct paper
//!   \[1\].
//

mod helpers;
mod trace;

use clap::Parser;
use error_trace::toplevel;
use helpers::ground_atom;
use humanlog::{DebugMode, HumanLogger};
use justact::collections::Recipient;
use justact::runtime::System as _;
use justact_prototype::agent::Agent;
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

    let mut st_antonius = Agent::with_store("st-antonius".into(), dataplane.scope("st-antonius"));
    st_antonius.program()
        // The St. Antonius will always publish they have the `patients` dataset.
        .state(Recipient::All, slick::parse::program(include_str!("./slick/st-antonius_1.slick")).unwrap().1)
        // And once they did so, they'll always try to enact- and write it.
        .enact_on_truth(ground_atom!(("st-antonius" "patients-2024") executed))
        .write((("st-antonius", "patients-2024"), "patients"), "st-antonius 1", b"billy bob jones\ncharlie brown\nanakin skywalker")

        // In the final example, we end with publishing some information useful for the
        // second agreement!
        .state_on_truth(ground_atom!((surf utils) involves surf), Recipient::All, slick::parse::program(include_str!("./slick/st-antonius_7.slick")).unwrap().1);

    let mut sync = Agent::new("consortium".into());
    sync.program()
        // In the final example, the consortium will refresh the agreement after St. Antonius published
        .agree(slick::parse::program(include_str!("./slick/consortium_1.slick")).unwrap().1)
        .wait_for_truth(ground_atom!(("st-antonius" "patients-2024") executed))
        .agree(slick::parse::program(include_str!("./slick/consortium_2.slick")).unwrap().1);

    // Run the runtime!
    let mut runtime = System::new();
    if let Err(err) = runtime.run::<Agent>([st_antonius], sync) {
        error!("{}", toplevel!(("Failed to run runtime"), err));
        std::process::exit(1);
    }

    // Done!
}
