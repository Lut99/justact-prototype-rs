//  WIRE.rs
//    by Lut99
//
//  Created:
//    13 Jan 2025, 15:11:30
//  Last edited:
//    15 Jan 2025, 16:30:42
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the concrete things on the wire - [`Action`]s and
//!   [`Message`]s.
//

use std::sync::Arc;

mod justact {
    pub use ::justact::actions::Action;
    pub use ::justact::agreements::Agreement;
    pub use ::justact::auxillary::{Actored, Authored, Identifiable, Timed};
    pub use ::justact::messages::{Message, MessageSet};
}


/***** TYPE ALIASES *****/
/// Fixes what it means to be an agreement.
pub type Agreement = justact::Agreement<Arc<Message>, u128>;





/***** LIBRARY *****/
/// Implements a [`Action`](justact::Action) in the prototype.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Action {
    /// Identifies this action (as an `(author, id)`-pair).
    pub id: (String, u32),

    /// The payload of the action.
    pub basis: justact::Agreement<Arc<Message>, u128>,
    /// The full justification.
    pub justification: justact::MessageSet<Arc<Message>>,
}
impl justact::Action for Action {
    type Message = Arc<Message>;

    #[inline]
    fn new(
        id: <Self::Id as ToOwned>::Owned,
        actor_id: <Self::ActorId as ToOwned>::Owned,
        basis: justact::Agreement<Self::Message, Self::Timestamp>,
        justification: justact::MessageSet<Self::Message>,
    ) -> Self
    where
        Self: Sized,
    {
        Self { id: (actor_id.to_owned(), id.to_owned().1), basis, justification }
    }

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
    fn actor_id(&self) -> &Self::ActorId { &self.id.0 }
}
impl justact::Identifiable for Action {
    type Id = (String, u32);

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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Message {
    /// Identifies this message (as an `(author, id)`-pair).
    pub id:      (String, u32),
    /// The payload of the message.
    pub payload: String,
}
impl justact::Authored for Message {
    type AuthorId = str;

    #[inline]
    fn author_id(&self) -> &Self::AuthorId { &self.id.0 }
}
impl justact::Identifiable for Message {
    type Id = (String, u32);

    #[inline]
    fn id(&self) -> &Self::Id { &self.id }
}
impl justact::Message for Message {
    type Payload = str;

    #[inline]
    fn new(id: <Self::Id as ToOwned>::Owned, author_id: <Self::AuthorId as ToOwned>::Owned, payload: <Self::Payload as ToOwned>::Owned) -> Self
    where
        Self: Sized,
    {
        Self { id: (author_id.to_owned(), id.to_owned().1), payload: payload.to_owned() }
    }

    #[inline]
    fn payload(&self) -> &Self::Payload { &self.payload }
}
