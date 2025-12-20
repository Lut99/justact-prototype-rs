//  SECTION 6.3.3 - OK.rs
//    by Lut99
//
//  Created:
//    30 Jan 2025, 18:28:07
//  Last edited:
//    31 Jan 2025, 18:15:38
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the third example from the paper.
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
    
    let mut bob = Agent::with_store("bob".into(), dataplane.scope("bob"));
    bob.program()
        // Bob publishes his workflow right from the start (`bob 1`).
        .state(Recipient::All, slick::parse::program(include_str!("./slick/bob_1.slick")).unwrap().1)
        // He can enact his workflow once the partners of it have confirmed their involvement.
        // Specifically, he's looking for confirmation that someone executes steps 2 and 3.
        .enact_on_truths([
            // `bob 1`
            ground_atom!((bob step1) executed), ground_atom!((bob step4) executed),
            // `st-antonius 1`
            ground_atom!(("st-antonius" "patients-2024") executed),
            // `st-antonius 4`
            ground_atom!((bob step3) executed),
            // `surf 1`
            ground_atom!((surf utils) has output "entry-count"),
            // `surf 2`
            ground_atom!((bob step2) executed),
        ])
        // Once the enactment is there, do step 1.
        .write((("bob", "step1"), "filter-consented"), "bob 6", b"code_that_actually_filters_consent_wowie();")
        // Then, once the partners have also written their dataset, it's our turn to do step 4.
        .wait_for_datum((("bob", "step3"), "num-consented"))
        .read((("bob", "step3"), "num-consented"), "bob 6");

    let mut st_antonius = Agent::with_store("st-antonius".into(), dataplane.scope("st-antonius"));
    st_antonius.program()
        /* Common */
        // The St. Antonius will always publish they have the `patients` dataset.
        .state(Recipient::All, slick::parse::program(include_str!("./slick/st-antonius_1.slick")).unwrap().1)
        // And once they did so, they'll always try to enact- and write it.
        .enact_on_truth(ground_atom!(("st-antonius" "patients-2024") executed))
        .write((("st-antonius", "patients-2024"), "patients"), "st-antonius 1", b"billy bob jones\ncharlie brown\nanakin skywalker")

        /* Scenario 1 */
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
        .state_on_truth(ground_atom!((amy end) executed), Recipient::All, slick::parse::program(include_str!("./slick/st-antonius_3.slick")).unwrap().1)
        
        /* Scenario 2 */
        // After Bob has published their workflow, the St. Antonius elects to do task 3,
        // giving SURF authorisation to do task 2 while at it.
        .state_on_truths([ground_atom!((bob step1) executed), ground_atom!((bob step4) executed)], Recipient::All, slick::parse::program(include_str!("./slick/st-antonius_4.slick")).unwrap().1)
        // Note that not just Bob needs to enact this action; St. Antonius needs to as well
        // to justify their own read! (It's not a valid effect, otherwise.)
        .enact_on_truths([
            // `bob 1`
            ground_atom!((bob step1) executed), ground_atom!((bob step4) executed),
            // `st-antonius 1`
            ground_atom!(("st-antonius" "patients-2024") executed),
            // `st-antonius 4`
            ground_atom!((bob step3) executed),
            // `surf 1`
            ground_atom!((surf utils) has output "entry-count"),
            // `surf 2`
            ground_atom!((bob step2) executed),
        ])
        .wait_for_data([(("surf", "utils"), "entry-count"), (("bob", "step2"), "consented")])
        .read((("surf", "utils"), "entry-count"), "st-antonius 7")
        .read((("bob", "step2"), "consented"), "st-antonius 7")
        .write((("bob", "step3"), "num-consented"), "st-antonius 7", b"2");

    let mut surf = Agent::with_store("surf".into(), dataplane.scope("surf"));
    surf.program()
        /* Common */
        // SURF publishes the existance of their utils package first.
        .state(Recipient::All, slick::parse::program(include_str!("./slick/surf_1.slick")).unwrap().1)
        // Then, once it's published, it enacts it and writes the data.
        .enact_on_truth(ground_atom!((surf utils) has output "entry-count"))
        .write((("surf", "utils"), "entry-count"), "surf 2", b"super_clever_code();")

        /* Scenario 2 */
        // In the second example, SURF will suggest to do the second step once Bob
        // publishes his workflow.
        .state_on_truths([ground_atom!((bob step1) executed), ground_atom!((bob step4) executed)], Recipient::All, slick::parse::program(include_str!("./slick/surf_2.slick")).unwrap().1)
        // Note that not just Bob needs to enact this action; SURF needs to as well to
        // justify their own read! (It's not a valid effect, otherwise.)
        .enact_on_truths([
            // `bob 1`
            ground_atom!((bob step1) executed), ground_atom!((bob step4) executed),
            // `st-antonius 1`
            ground_atom!(("st-antonius" "patients-2024") executed),
            // `st-antonius 4`
            ground_atom!((bob step3) executed),
            // `surf 1`
            ground_atom!((surf utils) has output "entry-count"),
            // `surf 2`
            ground_atom!((bob step2) executed),
        ])
        .wait_for_data([(("bob", "step1"), "filter-consented"), (("st-antonius", "patients-2024"), "patients")])
        .read((("bob", "step1"), "filter-consented"), "surf 5")
        .read((("st-antonius", "patients-2024"), "patients"), "surf 5")
        .write((("bob", "step2"), "consented"), "surf 5", b"billy bob jones\nanakin skywalker");

    let mut sync = Agent::new("consortium".into());
    sync.program().agree(slick::parse::program(include_str!("./slick/consortium_1.slick")).unwrap().1);

    // Run the runtime!
    let mut runtime = System::new();
    if let Err(err) = runtime.run::<Agent>([amy, bob, st_antonius, surf], sync) {
        error!("{}", toplevel!(("Failed to run runtime"), err));
        std::process::exit(1);
    }

    // Done!
}
