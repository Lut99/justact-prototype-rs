//  LIB.rs
//    by Lut99
//
//  Created:
//    15 Apr 2024, 16:13:37
//  Last edited:
//    21 Jan 2025, 11:01:47
//  Auto updated?
//    Yes
//
//  Description:
//!   Provides an implementation of a simple demo environment that simulates agents without threads or any of that fancy jazz.
//

// Declare modules
#[cfg(feature = "dataplane")]
pub mod dataplane;
pub mod io;
pub mod policy;
pub mod runtime;
pub mod sets;
pub mod wire;

// Use some of it
pub use runtime::{Error, Runtime};
