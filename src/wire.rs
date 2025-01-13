//  WIRE.rs
//    by Lut99
//
//  Created:
//    13 Jan 2025, 15:11:30
//  Last edited:
//    13 Jan 2025, 15:25:34
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the concrete things on the wire - [`Action`]s and
//!   [`Message`]s.
//

use std::sync::Arc;

use ::justact::agreements::Agreement;
use rand::Rng as _;
use rand::distributions::Alphanumeric;

mod justact {
    pub use ::justact::actions::Action;
    pub use ::justact::agreements::Agreement;
    pub use ::justact::auxillary::{Actored, Authored, Identifiable, Timed};
    pub use ::justact::messages::{Message, MessageSet};
}


/***** LIBRARY *****/
/// Implements a [`Action`](justact::Action) in the prototype.
#[derive(Clone, Debug)]
pub struct Action {
    /// Identifies this action.
    pub id: String,
    /// Identifies the actor of this action.
    pub actor_id: String,

    /// The payload of the action.
    pub basis: Agreement<Arc<Message>, u128>,
    /// The full justification.
    pub justification: justact::MessageSet<Arc<Message>>,
}
impl Action {
    /// Constructor for the Action that will choose a random ID.
    ///
    /// # Arguments
    /// - `actor_id`: The actor of this action.
    /// - `basis`: The agreement when this action was established.
    /// - `justification`: The payload of this action.
    ///
    /// # Returns
    /// A new Action ready to Justify Your Actions^{TM}.
    #[inline]
    pub fn new(
        actor_id: impl Into<String>,
        basis: impl Into<Agreement<Arc<Message>, u128>>,
        justification: impl Into<justact::MessageSet<Arc<Message>>>,
    ) -> Self {
        Self {
            id: rand::thread_rng().sample_iter(Alphanumeric).take(8).map(char::from).map(|c| c.to_ascii_lowercase()).collect::<String>(),
            actor_id: actor_id.into(),
            basis: basis.into(),
            justification: justification.into(),
        }
    }

    /// Constructor for the Action.
    ///
    /// # Arguments
    /// - `id`: The identifier of this message.
    /// - `actor_id`: The actor of this action.
    /// - `basis`: The agreement when this action was established.
    /// - `justification`: The payload of this action.
    ///
    /// # Returns
    /// A new Action ready to Justify Your Actions^{TM}.
    #[inline]
    pub fn with_id(
        id: impl Into<String>,
        actor_id: impl Into<String>,
        basis: impl Into<Agreement<Arc<Message>, u128>>,
        justification: impl Into<justact::MessageSet<Arc<Message>>>,
    ) -> Self {
        Self { id: id.into(), actor_id: actor_id.into(), basis: basis.into(), justification: justification.into() }
    }
}
impl justact::Action for Action {
    type Message = Arc<Message>;

    #[inline]
    fn basis(&self) -> &justact::Agreement<Self::Message, Self::Timestamp> { &self.basis }

    #[inline]
    fn justification(&self) -> &justact::MessageSet<Self::Message>
    where
        <Self::Message as justact::Identifiable>::Id: ToOwned,
    {
        &self.justification
    }
}
impl justact::Actored for Action {
    type ActorId = str;

    #[inline]
    fn actor_id(&self) -> &Self::ActorId { &self.actor_id }
}
impl justact::Identifiable for Action {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { &self.id }
}
impl justact::Timed for Action {
    type Timestamp = u128;

    #[inline]
    fn at(&self) -> &Self::Timestamp { &self.basis.at }
}

/// Implements a [`Message`](justact::Message) in the prototype.
#[derive(Clone, Debug)]
pub struct Message {
    /// Identifies this message.
    pub id: String,
    /// Identifies the author of the message.
    pub author_id: String,
    /// The payload of the message.
    pub payload: String,
}
impl Message {
    /// Constructor for the Message that will choose a random ID.
    ///
    /// # Arguments
    /// - `author_id`: The author of this message.
    /// - `payload`: The payload of this message.
    ///
    /// # Returns
    /// A new Message ready to Carry Policy.
    #[inline]
    pub fn new(author_id: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            id: rand::thread_rng().sample_iter(Alphanumeric).take(8).map(char::from).map(|c| c.to_ascii_lowercase()).collect::<String>(),
            author_id: author_id.into(),
            payload: payload.into(),
        }
    }

    /// Constructor for the Message.
    ///
    /// # Arguments
    /// - `id`: The identifier of this message.
    /// - `author_id`: The author of this message.
    /// - `payload`: The payload of this message.
    ///
    /// # Returns
    /// A new Message ready to Carry Policy.
    #[inline]
    pub fn with_id(id: impl Into<String>, author_id: impl Into<String>, payload: impl Into<String>) -> Self {
        Self { id: id.into(), author_id: author_id.into(), payload: payload.into() }
    }
}
impl justact::Authored for Message {
    type AuthorId = str;

    #[inline]
    fn author_id(&self) -> &Self::AuthorId { &self.author_id }
}
impl justact::Identifiable for Message {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { &self.id }
}
impl justact::Message for Message {
    type Payload = str;

    #[inline]
    fn payload(&self) -> &Self::Payload { &self.payload }
}
