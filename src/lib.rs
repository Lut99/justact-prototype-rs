//  LIB.rs
//    by Lut99
//
//  Created:
//    15 Apr 2024, 16:13:37
//  Last edited:
//    31 Jan 2025, 15:33:03
//  Auto updated?
//    Yes
//
//  Description:
//!   Provides an implementation of a simple demo environment that simulates agents without threads or any of that fancy jazz.
//

// Declare modules
pub mod auditing;
mod codegen;
#[cfg(feature = "dataplane")]
pub mod dataplane;
// pub mod events;
pub mod agent;
pub mod io;
pub mod policy;
pub mod runtime;
pub mod sets;
pub mod wire;

// Use some of it
pub use justact as spec;
pub use runtime::{Error, System};
