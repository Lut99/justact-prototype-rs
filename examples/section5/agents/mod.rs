//  MOD.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:50:19
//  Last edited:
//    15 Jan 2025, 15:38:29
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the agents used in the section 5 examples.
//

// Declare the agent modules
pub mod amdex;
pub mod consortium;

// Use the agents themselves
use std::task::Poll;

pub use amdex::Amdex;
pub use consortium::Consortium;
use thiserror::Error;

mod justact {
    pub use ::justact::actions::Action;
    pub use ::justact::actors::{Agent, View};
    pub use ::justact::agreements::Agreement;
    pub use ::justact::auxillary::Identifiable;
    pub use ::justact::collections::map::{Map, MapAsync};
    pub use ::justact::messages::Message;
    pub use ::justact::times::Times;
}


/***** ERRORS *****/
/// Defines an error abstracting over all agent errors.
#[derive(Debug, Error)]
pub enum Error {
    /// The `amdex` agent failed.
    #[error("The `amdex`-agent failed")]
    Amdex(#[source] amdex::Error),
}





/***** LIBRARY *****/
/// Super-agent abstracting over the individual agents.
pub enum Agent {
    Amdex(Amdex),
}
impl justact::Identifiable for Agent {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id {
        match self {
            Self::Amdex(a) => a.id(),
        }
    }
}
impl justact::Agent<(String, u32), (String, u32), str, u128> for Agent {
    type Error = Error;

    #[inline]
    fn poll<T, A, S, E, SM, SA>(&mut self, view: justact::View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: justact::Times<Timestamp = u128>,
        A: justact::Map<justact::Agreement<SM, u128>>,
        S: justact::MapAsync<Self::Id, SM>,
        E: justact::MapAsync<Self::Id, SA>,
        SM: justact::Message<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: justact::Action<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u128>,
    {
        match self {
            Self::Amdex(a) => a.poll(view).map_err(Error::Amdex),
        }
    }
}
impl From<Amdex> for Agent {
    #[inline]
    fn from(value: Amdex) -> Self { Self::Amdex(value) }
}
