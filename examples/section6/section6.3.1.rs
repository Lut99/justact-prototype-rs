//  SECTION 6.3.1.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:49:57
//  Last edited:
//    31 Jan 2025, 18:14:36
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the first example from section 5.4 of the JustAct paper
//!   \[1\].
//

// mod agents;
mod helpers;
mod trace;

// use agents::{Agent, Amy, Consortium, Dan, Script, StAntonius, Surf};
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

    let mut amy = Agent::new("amy".into());
    amy.program()
        // In the first scenario, Amy publishes her execution of `entry-count` on the St.
        // Antonius' dataset.
        // She only does that once she knows the package exists. As such, she waits until she
        // sees: `(surf utils) ready.` before she publishes `amy 1`.
        .state_on_truth(ground_atom!((surf utils) executed), Recipient::All, slick::parse::program(include_str!("./slick/amy_1.slick")).unwrap().1)
        // Then she waits until the St. Antonius has executed her task. Once so, she publishes
        // her intent to download the result (`amy 2`).
        .state_on_truth(
            ground_atom!((amy "count-patients") executed),
            Recipient::All, slick::parse::program(include_str!("./slick/amy_2.slick")).unwrap().1,
        )
        // Finally, once she's gotten St. Antonius' authorisation to execute `amy 2`, she'll
        // collect the agreement and all statements (except Dan's) and enact it.
        .enact_on_truths([
            // `amy 1`
            ground_atom!((amy "count-patients") has output "num-patients"),
            // `amy 2`
            ground_atom!((amy end) executed),
            // `st antonius 1`
            ground_atom!(("st-antonius" "patients-2024") executed),
            // `st antonius 2`
            ground_atom!((amy "count-patients") executed),
            // `st antonius 3`
            ground_atom!(authorise read of ((amy "count-patients") "num-patients") for (amy end) by amy),
            // `surf 1`
            ground_atom!((surf utils) has output "entry-count")
        ]);

    let mut st_antonius = Agent::with_store("st-antonius".into(), dataplane.scope("st-antonius"));
    st_antonius.program()
        // The St. Antonius will always publish they have the `patients` dataset.
        .state(Recipient::All, slick::parse::program(include_str!("./slick/st-antonius_1.slick")).unwrap().1)
        // And once they did so, they'll always try to enact- and write it.
        .enact_on_truth(ground_atom!(("st-antonius" "patients-2024") executed))
        .write((("st-antonius", "patients-2024"), "patients"), "st-antonius 1", b"billy bob jones\ncharlie brown\nanakin skywalker")

        // After Amy has put a task up for grabs, the St. Antonius will do it themselves.
        .state_on_truth(ground_atom!((amy "count-patients") has output "num-patients"), Recipient::All, slick::parse::program(include_str!("./slick/st-antonius_2.slick")).unwrap().1)
        // Then the St. Antonius will enact its own statement, reading and writing as appropriate.
        .enact_on_truths([
            // `amy 1`
            ground_atom!((amy "count-patients") has output "num-patients"),
            // `st antonius 1`
            ground_atom!(("st-antonius" "patients-2024") executed),
            // `st antonius 2`
            ground_atom!((amy "count-patients") executed),
            // `surf 1`
            ground_atom!((surf utils) has output "entry-count")
        ])
        .read((("surf", "utils"), "entry-count"), "st-antonius 3")
        .read((("st-antonius", "patients-2024"), "patients"), "st-antonius 3")
        .write((("amy", "count-patients"), "num-patients"), "st-antonius 3", b"3")

        // Eventually, Amy will have published her request to download. Which we authorise.
        .state_on_truth(ground_atom!((amy end) executed), Recipient::All, slick::parse::program(include_str!("./slick/st-antonius_3.slick")).unwrap().1);

    let mut surf = Agent::with_store("surf".into(), dataplane.scope("surf"));
    surf.program()
        // SURF publishes the existance of their utils package first.
        .state(Recipient::All, slick::parse::program(include_str!("./slick/surf_1.slick")).unwrap().1)
        // Then, once it's published, it enacts it and writes the data.
        .enact_on_truth(ground_atom!((surf utils) has output "entry-count"))
        .write((("surf", "utils"), "entry-count"), "surf 2", b"super_clever_code();");

    let mut sync = Agent::new("consortium".into());
    sync.program().agree(slick::parse::program(include_str!("./slick/consortium_1.slick")).unwrap().1);

    // Run the runtime!
    let mut runtime = System::new();
    if let Err(err) = runtime.run::<Agent>([amy, st_antonius, surf], sync) {
        error!("{}", toplevel!(("Failed to run runtime"), err));
        std::process::exit(1);
    }

    // Done!
}
