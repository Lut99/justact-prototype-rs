//  MAIN.rs
//    by Lut99
//
//  Created:
//    16 Jan 2025, 11:27:25
//  Last edited:
//    23 Jan 2025, 13:05:04
//  Auto updated?
//    Yes
//
//  Description:
//!   Entrypoint for the `inspector` binary.
//

mod app;
mod trace;
mod widgets;

use std::io::{Result as IResult, Write};

use app::App;
use clap::Parser;
use error_trace::{ErrorTrace as _, trace};
use humanlog::{ColourChoice, DebugMode, HumanLogger, LogWriter};
use log::{Level, debug, error, info};
use parking_lot::lock_api::RawMutex as _;
use parking_lot::{Mutex, RawMutex};
use tokio::fs::File;
use tokio::io::{AsyncRead, stdin};


/***** GLOBALS *****/
/// Buffers any [`log`] write to stderr.
static STDERR_BUF: LockedWriter = LockedWriter(Mutex::const_new(RawMutex::INIT, Vec::new()));





/***** HELPERS *****/
/// Wraps around a buffer to make it writable but immutably.
struct LockedWriter(Mutex<Vec<u8>>);
impl Write for &'static LockedWriter {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> IResult<usize> {
        let mut target = self.0.lock();
        target.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> IResult<()> {
        let mut target = self.0.lock();
        target.flush()
    }
}





/***** ARGUMENTS *****/
#[derive(Parser)]
struct Arguments {
    /// If given, enables additional INFO- and DEBUG-level log statements.
    #[clap(long, global = true)]
    debug: bool,
    /// If given, enables additional TRACE-level log statements. Implies `--debug`.
    #[clap(long, global = true)]
    trace: bool,

    /// If given, denotes the file to read the traces from. Use `-` to read from stdout instead.
    #[clap(name = "PATH", default_value = "-")]
    path: String,
}





/***** ENTRYPOINT *****/
#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Parse the args
    let args = Arguments::parse();

    // Setup the logger
    if let Err(err) = HumanLogger::new(
        [LogWriter::new(
            &STDERR_BUF,
            ColourChoice::Yes,
            [Level::Trace, Level::Debug, Level::Info, Level::Warn, Level::Error],
            "delayed stderr writer",
        )],
        DebugMode::from_flags(args.trace, args.debug),
    )
    .init()
    {
        eprintln!("WARNING: Failed to setup logger: {err} (logging disabled for this session)");
    }
    info!("{} - v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));

    // Open the files to read
    let (what, handle): (String, Box<dyn 'static + Send + AsyncRead + Unpin>) = if args.path == "-" {
        debug!("Opening stdout...");
        ("stdout".into(), Box::new(stdin()))
    } else {
        debug!("Opening input file {:?}...", args.path);
        (format!("{:?}", args.path), match File::open(&args.path).await {
            Ok(handle) => Box::new(handle),
            Err(err) => {
                error!("{}", trace!(("Failed to open input trace file {:?}", args.path), err));
                eprintln!("{}", String::from_utf8_lossy(&STDERR_BUF.0.lock()));
                std::process::exit(1);
            },
        })
    };

    // Now run the app
    debug!("Entering main game loop");
    if let Err(err) = App::new(what, handle).run().await {
        error!("{}", err.trace());
        eprintln!("{}", String::from_utf8_lossy(&STDERR_BUF.0.lock()));
        std::process::exit(1);
    }
    eprintln!("{}", String::from_utf8_lossy(&STDERR_BUF.0.lock()));
    println!("Done.");
}
