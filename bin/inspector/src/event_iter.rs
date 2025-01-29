//  TRACE.rs
//    by Lut99
//
//  Created:
//    16 Jan 2025, 11:43:30
//  Last edited:
//    29 Jan 2025, 21:44:01
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines a reader for trace files.
//

use std::io::ErrorKind;

use justact_prototype::auditing::Event;
use log::debug;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt as _, BufReader};


/***** ERRORS *****/
/// Defines errors yielded by the [`BraceIter`].
#[derive(Debug, Error)]
pub enum Error {
    #[error("{}:{}: Failed to deserialize event", pos.0, pos.1)]
    EventDeserialize {
        pos: (usize, usize),
        #[source]
        err: serde_json::Error,
    },
    #[error("{}:{}: Expected closing brance '}}' for opening brace at {}:{}", close.0, close.1, open.0, open.1 )]
    MissingClosingBrace { open: (usize, usize), close: (usize, usize) },
    #[error("Failed to read from {what}")]
    ReaderRead {
        what: String,
        #[source]
        err:  std::io::Error,
    },
    #[error("{}:{}: Encountered unexpected character {c:?}", pos.0, pos.1)]
    UnexpectedChar { pos: (usize, usize), c: String },
}





/***** LIBRARY *****/
/// Iterator that will read chunks wrapped in `{}` from a given `R`eader.
///
/// Will generate errors if other things were found in between that aren't whitespaces, or those
/// things in braces aren't [`Event`]s.
pub struct EventIter<R> {
    /// Some description of what we're reading.
    what:   String,
    /// The reader to read from.
    reader: BufReader<R>,
    /// The current line/col pos.
    pos:    (usize, usize),
}

// Constructors
impl<R> EventIter<R>
where
    R: AsyncRead + Unpin,
{
    /// Constructor for the EventIter.
    ///
    /// # Arguments
    /// - `what`: Some name (path or otherwise) that describes the `input` (used for debugging purposes only).
    /// - `input`: Some [`Read`]er from which to read [`Event`]s.
    ///
    /// # Returns
    /// A new BraceIter that will yield every pair of curly braces in the input text, or errors
    /// otherwise.
    #[inline]
    pub fn new(what: String, input: R) -> Self { Self { what, reader: BufReader::new(input), pos: (1, 0) } }
}

// Reading
impl<R> EventIter<R>
where
    R: AsyncRead + Unpin,
{
    /// Reads a single character.
    ///
    /// # Returns
    /// The read character, as a `char`.
    ///
    /// # Errors
    /// This function may error if it failed to read using the backend `R`eader.
    async fn read_char(&mut self) -> Result<Option<char>, Error> {
        // Read the character
        let mut c: [u8; 1] = [0];
        match self.reader.read_exact(&mut c).await {
            Ok(0) => Ok(None),
            Ok(_) => {
                // Update the pos before returning
                let c: char = c[0] as char;
                if c == '\n' {
                    self.pos.0 += 1;
                    self.pos.1 = 1;
                } else {
                    self.pos.1 += 1;
                }
                Ok(Some(c))
            },
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => Ok(None),
            Err(err) => return Err(Error::ReaderRead { what: self.what.clone(), err }),
        }
    }
}

// Iteration
impl<R> EventIter<R>
where
    R: AsyncRead + Unpin,
{
    /// Yields the next item in the iterator, as long as supply lasts.
    pub async fn next(&mut self) -> Option<Result<Event<'static>, Error>> {
        // Start to search for the next '{'
        let mut buf: String = String::with_capacity(32);
        loop {
            // Get the next char
            match self.read_char().await {
                // We found the start of a brace; do nesting-aware search of the closing brace
                Ok(Some('{')) => {
                    buf.push('{');
                    let open_pos: (usize, usize) = self.pos;
                    let mut depth: usize = 1;
                    loop {
                        match self.read_char().await {
                            // What to do on braces
                            Ok(Some('{')) => {
                                depth += 1;
                                if buf.len() == buf.capacity() {
                                    buf.reserve(buf.capacity());
                                }
                                buf.push('{');
                            },
                            Ok(Some('}')) => {
                                depth -= 1;
                                if buf.len() == buf.capacity() {
                                    buf.reserve(buf.capacity());
                                }
                                buf.push('}');

                                // If we have parity, we have a (potential) trace!
                                if depth == 0 {
                                    debug!("Found raw trace: {buf:?}");
                                    match serde_json::from_str::<Event<'static>>(&buf) {
                                        Ok(trace) => return Some(Ok(trace)),
                                        Err(err) => return Some(Err(Error::EventDeserialize { pos: open_pos, err })),
                                    }
                                }
                            },

                            // Other bytes are just appended to the buffer
                            Ok(Some(c)) => {
                                if buf.len() == buf.capacity() {
                                    buf.reserve(buf.capacity());
                                }
                                buf.push(c);
                            },

                            // No input / errors
                            Ok(None) => return Some(Err(Error::MissingClosingBrace { open: open_pos, close: self.pos })),
                            Err(err) => return Some(Err(err)),
                        }
                    }
                },

                // Else, we accept whitespace, but nothing more
                Ok(Some(c)) if c.is_whitespace() => continue,
                Ok(Some(c)) => return Some(Err(Error::UnexpectedChar { pos: self.pos, c: c.into() })),

                // No input / errors
                Ok(None) => return None,
                Err(err) => return Some(Err(err)),
            }
        }
    }
}
