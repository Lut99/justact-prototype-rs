//  TRACE.rs
//    by Lut99
//
//  Created:
//    15 Jan 2025, 17:51:09
//  Last edited:
//    31 Jan 2025, 18:13:45
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a cross-example trace handler.
//

use std::error::Error;
use std::fs::File;
use std::io::Write as _;
use std::path::Path;

use justact_prototype::auditing::Event;
use justact_prototype::io::EventHandler;


/***** LIBRARY *****/
/// An [`EventHandler`] that writes to stdout.
pub struct StdoutEventHandler;
impl EventHandler for StdoutEventHandler {
    #[inline]
    fn handle(&mut self, event: Event) -> Result<(), Box<dyn 'static + Send + Error>> {
        println!("{}", serde_json::to_string(&event).map_err(|err| -> Box<dyn 'static + Send + Error> { Box::new(err) })?);
        Ok(())
    }
}



/// An [`EventHandler`] that writes to a file.
pub struct FileEventHandler {
    /// The file handle to write to.
    handle: File,
}
impl FileEventHandler {
    /// Constructor for the FileEventHandler that opens a file.
    ///
    /// # Arguments
    /// - `path`: The path of the file to create.
    ///
    /// # Returns
    /// A new FileEventHandler that can be used to log events.
    ///
    /// # Errors
    /// This function fails if we failed to create a file at the given `path`.
    #[inline]
    pub fn new(path: impl AsRef<Path>) -> Result<Self, std::io::Error> { Ok(Self { handle: File::create(path)? }) }
}
impl EventHandler for FileEventHandler {
    #[inline]
    fn handle(&mut self, event: Event) -> Result<(), Box<dyn 'static + Send + std::error::Error>> {
        self.handle
            .write_all(&serde_json::to_string(&event).map_err(|err| -> Box<dyn 'static + Send + Error> { Box::new(err) })?.as_bytes())
            .map_err(|err| -> Box<dyn 'static + Send + Error> { Box::new(err) })
    }
}
