//  ERROR.rs
//    by Lut99
//
//  Created:
//    21 Jan 2025, 09:31:53
//  Last edited:
//    29 Jan 2025, 22:37:08
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines a custom error type that makes working with errors in agents
//!   convenient (FINALLY).
//

use std::error;
use std::fmt::{Display, Formatter, Result as FResult};


/***** LIBRARY *****/
/// Extends [`Result<T, impl Error>`](Result) with the ability to be easily cast into an [`Error`].
pub trait ResultToError<T> {
    /// Casts this error into an [`Error`].
    ///
    /// # Returns
    /// An [`Error`] that implements [`Error`](error::Error).
    fn cast(self) -> Result<T, Error>;
}
impl<T, E: 'static + Send + error::Error> ResultToError<T> for Result<T, E> {
    #[inline]
    fn cast(self) -> Result<T, Error> { self.map_err(|err| Error(Box::new(err))) }
}



/// Wraps a [`Box<dyn Error>`](Box) such that the type itself also implements
/// [`Error`](error::Error).
#[derive(Debug)]
pub struct Error(Box<dyn 'static + Send + error::Error>);
impl Error {
    /// Constructor for the Error from a generic [`Error`](error::Error).
    ///
    /// # Arguments
    /// - `err`: The [`Error`](error::Error) to wrap.
    ///
    /// # Returns
    /// A new Error, ready to wreak havoc.
    #[inline]
    #[allow(unused)]
    pub fn new(err: impl 'static + Send + error::Error) -> Self { Self(Box::new(err)) }
}
impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult { self.0.fmt(f) }
}
impl error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> { self.0.source() }
}
