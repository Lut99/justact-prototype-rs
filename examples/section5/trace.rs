//  TRACE.rs
//    by Lut99
//
//  Created:
//    15 Jan 2025, 17:51:09
//  Last edited:
//    29 Jan 2025, 22:36:13
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a cross-example trace handler.
//

use std::error::Error;

use justact_prototype::auditing::Event;
use justact_prototype::io::EventHandler;


/***** LIBRARY *****/
/// An [`EventHandler`] that writes to stdout.
pub struct StdoutEventHandler;
impl EventHandler for StdoutEventHandler {
    #[inline]
    fn handle(&self, event: Event) -> Result<(), Box<dyn 'static + Send + Error>> {
        println!("{}", serde_json::to_string(&event).map_err(|err| -> Box<dyn 'static + Send + Error> { Box::new(err) })?);
        Ok(())
    }
}
