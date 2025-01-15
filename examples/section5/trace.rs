//  TRACE.rs
//    by Lut99
//
//  Created:
//    15 Jan 2025, 17:51:09
//  Last edited:
//    15 Jan 2025, 17:52:56
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a cross-example trace handler.
//

use std::error::Error;

use justact_prototype::io::{Trace, TraceHandler};


/***** LIBRARY *****/
/// A [`TraceHandler`] that writes to stdout.
pub struct StdoutTraceHandler;
impl TraceHandler for StdoutTraceHandler {
    #[inline]
    fn handle(&self, trace: Trace) -> Result<(), Box<dyn Error>> {
        println!("{}", serde_json::to_string(&trace).map_err(Box::new)?);
        Ok(())
    }
}
