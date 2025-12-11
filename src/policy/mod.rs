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

// Imports
use std::convert::Infallible;
use std::error::Error;


/***** LIBRARY *****/
/// A generic policy trait that allows us to serialize a policy to a string.
pub trait PolicySerialize {
    /// Serializes this policy to a string.
    ///
    /// This should happens such that a reverse [`PolicyDeserialize::deserialize()`]-implementation
    /// can recover an equivalent policy.
    ///
    /// # Returns
    /// A [`String`] encoding the policy.
    fn serialize(&self) -> String;
}

// Std impls
impl PolicySerialize for str {
    #[inline]
    fn serialize(&self) -> String { self.to_string() }
}



/// A generic policy trait that allows us to deserialize a string back into a policy.
pub trait PolicyDeserialize<'a>: ToOwned {
    /// The error casted by [`PolicyDeserialize::deserialize()`].
    type Error: Error;

    /// Deserializes this policy from a string.
    ///
    /// This function should typically be able to recover an equivalent policy to the original one
    /// serialized with [`PolicySerialize::serialize()`].
    ///
    /// # Arguments
    /// - `raw`: The serialized policy to deserialize.
    ///
    /// # Returns
    /// An instance of (owned) Self that is the deserialized policy.
    ///
    /// # Errors
    /// This function can fail with a [`PolicyDeserialize::Error`] if the input was not valid for
    /// this policy type.
    fn deserialize(raw: &'a str) -> Result<Self::Owned, Self::Error>;
}

// Std impls
impl<'a> PolicyDeserialize<'a> for str {
    type Error = Infallible;

    #[inline]
    fn deserialize(raw: &'a str) -> Result<Self::Owned, Self::Error> { Ok(raw.to_string()) }
}
