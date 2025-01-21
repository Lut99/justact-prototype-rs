//  MOD.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:50:19
//  Last edited:
//    21 Jan 2025, 09:58:53
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the agents used in the section 5 examples.
//

// Declare the agent modules
pub mod amdex;
pub mod amy;
pub mod consortium;
pub mod dan;
pub mod st_antonius;

// Use the agents themselves
use std::task::Poll;

pub use amdex::Amdex;
pub use amy::Amy;
pub use consortium::Consortium;
pub use dan::Dan;
pub use st_antonius::StAntonius;
use thiserror::Error;

mod justact {
    pub use ::justact::actions::ConstructableAction;
    pub use ::justact::actors::{Agent, View};
    pub use ::justact::agreements::Agreement;
    pub use ::justact::auxillary::Identifiable;
    pub use ::justact::collections::map::{Map, MapAsync};
    pub use ::justact::messages::ConstructableMessage;
    pub use ::justact::times::Times;
}


/***** ERRORS *****/
/// Defines an error abstracting over all agent errors.
#[derive(Debug, Error)]
pub enum Error {
    /// The `amdex` agent failed.
    #[error("The `amdex`-agent failed")]
    Amdex(#[source] amdex::Error),
    /// The `amy` agent failed.
    #[error("The `amy`-agent failed")]
    Amy(#[source] amy::Error),
    /// The `dan` agent failed.
    #[error("The `dan`-agent failed")]
    Dan(#[source] dan::Error),
    /// The `st-antonius` agent failed.
    #[error("The `st-antonius`-agent failed")]
    StAntonius(#[source] st_antonius::Error),
}





/***** AGENT HELPER FUNCTIONS *****/
/// Creates a message of type `SM`.
///
/// This is done through a helper message to avoid the awkward double author ID.
///
/// # Arguments
/// - `id`: The identifier of the message.
/// - `author_id`: The identifier of the message's author.
/// - `payload`: The actual payload of the message.
///
/// # Returns
/// A new message of type `SM`, constructed with its
/// [`ConstructableMessage`](justact::ConstructableMessage) implementation.
fn create_message<SM>(id: u32, author_id: impl Into<String>, payload: impl Into<<SM::Payload as ToOwned>::Owned>) -> SM
where
    SM: justact::ConstructableMessage<Id = (String, u32), AuthorId = str>,
    SM::Payload: ToOwned,
{
    SM::new((String::new(), id), author_id.into(), payload.into())
}





/***** LIBRARY *****/
/// Super-agent abstracting over the individual agents.
pub enum Agent {
    Amdex(Amdex),
    Amy(Amy),
    Dan(Dan),
    StAntonius(StAntonius),
}
impl justact::Identifiable for Agent {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id {
        match self {
            Self::Amdex(a) => a.id(),
            Self::Amy(a) => a.id(),
            Self::Dan(d) => d.id(),
            Self::StAntonius(s) => s.id(),
        }
    }
}
impl justact::Agent<(String, u32), (String, u32), str, u64> for Agent {
    type Error = Error;

    #[inline]
    fn poll<T, A, S, E, SM, SA>(&mut self, view: justact::View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: justact::Times<Timestamp = u64>,
        A: justact::Map<justact::Agreement<SM, u64>>,
        S: justact::MapAsync<Self::Id, SM>,
        E: justact::MapAsync<Self::Id, SA>,
        SM: justact::ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: justact::ConstructableAction<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        match self {
            Self::Amdex(a) => a.poll(view).map_err(Error::Amdex),
            Self::Amy(a) => a.poll(view).map_err(Error::Amy),
            Self::Dan(d) => d.poll(view).map_err(Error::Dan),
            Self::StAntonius(s) => s.poll(view).map_err(Error::StAntonius),
        }
    }
}
impl From<Amdex> for Agent {
    #[inline]
    fn from(value: Amdex) -> Self { Self::Amdex(value) }
}
impl From<Amy> for Agent {
    #[inline]
    fn from(value: Amy) -> Self { Self::Amy(value) }
}
impl From<Dan> for Agent {
    #[inline]
    fn from(value: Dan) -> Self { Self::Dan(value) }
}
impl From<StAntonius> for Agent {
    #[inline]
    fn from(value: StAntonius) -> Self { Self::StAntonius(value) }
}
