//  MOD.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:50:19
//  Last edited:
//    31 Jan 2025, 15:54:50
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the agents used in the section 5 examples.
//

// Declare the agent modules
// pub mod amy;
// pub mod bob;
// pub mod consortium;
// pub mod dan;
// pub mod st_antonius;
// pub mod surf;

// Use the agents themselves
use std::hash::Hash;
use std::task::Poll;

// pub use amy::Amy;
// pub use bob::Bob;
// pub use consortium::Consortium;
// pub use dan::Dan;
use slick::Program;
// pub use st_antonius::StAntonius;
// pub use surf::Surf;
use thiserror::Error;

mod justact {
    pub use ::justact::actions::ConstructableAction;
    pub use ::justact::actors::{Agent, View};
    pub use ::justact::auxillary::{Authored, Identifiable};
    pub use ::justact::collections::set::{InfallibleSetSync, Set, SetAsync};
    pub use ::justact::messages::{ConstructableMessage, Message, MessageSet};
}


/***** ERRORS *****/
/// Defines an error abstracting over all agent errors.
#[derive(Debug, Error)]
pub enum Error {
    // /// The `amy` agent failed.
    // #[error("The `amy`-agent failed")]
    // Amy(#[source] amy::Error),
    // /// The `bob` agent failed.
    // #[error("The `bob`-agent failed")]
    // Bob(#[source] bob::Error),
    // /// The `dan` agent failed.
    // #[error("The `dan`-agent failed")]
    // Dan(#[source] dan::Error),
    // /// The `st-antonius` agent failed.
    // #[error("The `st-antonius`-agent failed")]
    // StAntonius(#[source] st_antonius::Error),
    // /// The `surf` agent failed.
    // #[error("The `surf`-agent failed")]
    // Surf(#[source] surf::Error),
}





/***** AGENT HELPER FUNCTIONS *****/
/// Creates a message of type `SM`.
///
/// # Arguments
/// - `author_id`: The identifier of the message's author.
/// - `payload`: The actual payload of the message.
///
/// # Returns
/// A new message of type `SM`, constructed with its
/// [`ConstructableMessage`](justact::ConstructableMessage) implementation.
fn create_message<SM>(author_id: impl Into<String>, payload: impl Into<<SM::Payload as ToOwned>::Owned>) -> SM
where
    SM: justact::ConstructableMessage<AuthorId = str>,
    SM::Payload: ToOwned,
{
    SM::new(author_id.into(), payload.into())
}

/// Creates a message of type `SA`.
///
/// This is done through a helper function to avoid the awkward double author ID.
///
/// # Arguments
/// - `actor_id`: The identifier of the action's actor. Needn't be the same as the author of any
///   message.
/// - `basis`: The basis of the action. Note that **this will be automatically injected in the `just`ification.**
/// - `extra`: The justification for this action. The `basis` will automatically be injected into this.
///
/// # Returns
/// A new message of type `SA`, constructed with its
/// [`ConstructableAction`](justact::ConstructableAction) implementation.
fn create_action<SA>(actor_id: impl Into<String>, basis: impl Into<SA::Message>, extra: impl Into<justact::MessageSet<SA::Message>>) -> SA
where
    SA: justact::ConstructableAction<ActorId = str>,
    SA::Message: Clone + justact::Message,
    <SA::Message as justact::Authored>::AuthorId: ToOwned,
{
    let basis: SA::Message = basis.into();
    let extra: justact::MessageSet<SA::Message> = extra.into();

    // Now create the action
    SA::new(actor_id.into(), basis, extra)
}





/***** AUXILLARY *****/
/// Defines which script agents will execute.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Script {
    /// The first example, that of section 6.3.1.
    #[allow(unused)]
    Section6_3_1,
    /// The second example, that of section 6.3.2.
    #[allow(unused)]
    Section6_3_2,
    /// The third example, that of section 6.3.3. But now Amy doesn't die.
    #[allow(unused, non_camel_case_types)]
    Section6_3_3_ok,
    /// The third example, that of section 6.3.3. But now Amy DOES die.
    #[allow(unused, non_camel_case_types)]
    Section6_3_3_crash,
    /// The fourth example, that of section 6.3.4.
    #[allow(unused)]
    Section6_3_4,
    /// The fifth example, that of section 6.3.5.
    #[allow(unused)]
    Section6_3_5,
}





/***** LIBRARY *****/
/// Super-agent abstracting over the individual agents.
pub enum Agent {
    // Amy(Amy),
    // Bob(Bob),
    // Dan(Dan),
    // StAntonius(StAntonius),
    // Surf(Surf),
}
impl justact::Identifiable for Agent {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id {
        match self {
            Self::Amy(a) => a.id(),
            // Self::Bob(b) => b.id(),
            // Self::Dan(d) => d.id(),
            // Self::StAntonius(s) => s.id(),
            // Self::Surf(s) => s.id(),
        }
    }
}
impl justact::Agent<Program> for Agent {
    type Error = Error;

    #[inline]
    fn poll<A, S, E, SM, SA>(&mut self, view: justact::View<A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        A: justact::Set<SM>,
        S: justact::SetAsync<Self::Id, SM>,
        E: justact::SetAsync<Self::Id, SA>,
        SM: justact::ConstructableMessage<AuthorId = Self::Id, Payload = Program>,
        SA: justact::ConstructableAction<ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        match self {
            Self::Amy(a) => a.poll(view).map_err(Error::Amy),
            // Self::Bob(b) => b.poll(view).map_err(Error::Bob),
            // Self::Dan(d) => d.poll(view).map_err(Error::Dan),
            // Self::StAntonius(s) => s.poll(view).map_err(Error::StAntonius),
            // Self::Surf(s) => s.poll(view).map_err(Error::Surf),
        }
    }
}
impl From<Amy> for Agent {
    #[inline]
    fn from(value: Amy) -> Self { Self::Amy(value) }
}
// impl From<Bob> for Agent {
//     #[inline]
//     fn from(value: Bob) -> Self { Self::Bob(value) }
// }
// impl From<Dan> for Agent {
//     #[inline]
//     fn from(value: Dan) -> Self { Self::Dan(value) }
// }
// impl From<StAntonius> for Agent {
//     #[inline]
//     fn from(value: StAntonius) -> Self { Self::StAntonius(value) }
// }
// impl From<Surf> for Agent {
//     #[inline]
//     fn from(value: Surf) -> Self { Self::Surf(value) }
// }
