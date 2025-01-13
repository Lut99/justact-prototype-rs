//  MOD.rs
//    by Lut99
//
//  Created:
//    26 Nov 2024, 11:53:46
//  Last edited:
//    13 Jan 2025, 11:55:18
//  Auto updated?
//    Yes
//
//  Description:
//!   Provides implementations of the JustAct framework for various policy
//!   languages.
//

// Declare the modules
#[cfg(feature = "datalog")]
pub mod datalog;
#[cfg(feature = "slick")]
pub mod slick;
