//  WIRE.rs
//    by Lut99
//
//  Created:
//    13 Jan 2025, 15:11:30
//  Last edited:
//    24 Jan 2025, 22:41:34
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the concrete things on the wire - [`Action`]s and
//!   [`Message`]s.
//

use std::convert::Infallible;
use std::sync::Arc;

use rand::Rng as _;
mod justact {
    pub use ::justact::actions::{Action, ConstructableAction};
    pub use ::justact::agreements::Agreement;
    pub use ::justact::auxillary::{Actored, Authored, Identifiable, Timed};
    pub use ::justact::collections::map::{InfallibleMapSync, Map};
    pub use ::justact::messages::{ConstructableMessage, Message, MessageSet};
}


/***** TYPE ALIASES *****/
/// Fixes what it means to be an agreement.
pub type Agreement = justact::Agreement<Arc<Message>, u64>;





/***** LIBRARY *****/
/// Implements a [`Action`](justact::Action) in the prototype.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Action {
    /// Identifies this action (as an `(author, id)`-pair).
    pub id: (String, char),

    /// The payload of the action.
    pub basis: Agreement,
    /// The full justification.
    pub extra: justact::MessageSet<Arc<Message>>,
}
impl justact::ConstructableAction for Action {
    #[inline]
    fn new(
        id: <Self::Id as ToOwned>::Owned,
        actor_id: <Self::ActorId as ToOwned>::Owned,
        basis: justact::Agreement<Self::Message, Self::Timestamp>,
        extra: justact::MessageSet<Self::Message>,
    ) -> Self
    where
        Self: Sized,
    {
        Self { id: (actor_id.to_owned(), id.to_owned().1), basis, extra }
    }
}
impl justact::Action for Action {
    type Message = Arc<Message>;

    #[inline]
    fn basis(&self) -> &justact::Agreement<Self::Message, Self::Timestamp> { &self.basis }

    #[inline]
    fn extra(&self) -> &justact::MessageSet<Self::Message>
    where
        <Self::Message as justact::Identifiable>::Id: ToOwned,
    {
        &self.extra
    }

    #[inline]
    fn payload(&self) -> justact::MessageSet<Self::Message>
    where
        <Self::Message as justact::Identifiable>::Id: ToOwned,
    {
        use justact::{ConstructableMessage as _, InfallibleMapSync as _};

        let author = &self.id.0;
        let mut res = self.extra().clone();
        res.add(self.basis.message.clone());
        res.add(Arc::new(Message::new((author.clone(), rand::rng().random::<u32>()), author.clone(), "".into())));
        res
    }
}
impl justact::Actored for Action {
    type ActorId = str;

    #[inline]
    fn actor_id(&self) -> &Self::ActorId { &self.id.0 }
}
impl justact::Identifiable for Action {
    type Id = (String, char);

    #[inline]
    fn id(&self) -> &Self::Id { &self.id }
}
impl justact::Timed for Action {
    type Timestamp = u64;

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
impl justact::ConstructableMessage for Message {
    #[inline]
    fn new(id: <Self::Id as ToOwned>::Owned, author_id: <Self::AuthorId as ToOwned>::Owned, payload: <Self::Payload as ToOwned>::Owned) -> Self
    where
        Self: Sized,
    {
        Self { id: (author_id.to_owned(), id.to_owned().1), payload: payload.to_owned() }
    }
}
impl justact::Message for Message {
    type Payload = str;

    #[inline]
    fn payload(&self) -> &Self::Payload { &self.payload }
}
impl justact::Map<Self> for Message {
    type Error = Infallible;

    #[inline]
    fn contains_key(&self, id: &<Self as justact::Identifiable>::Id) -> Result<bool, Self::Error>
    where
        Self: justact::Identifiable,
    {
        Ok(&self.id == id)
    }

    #[inline]
    fn get(&self, id: &<Self as justact::Identifiable>::Id) -> Result<Option<&Self>, Self::Error>
    where
        Self: justact::Identifiable,
    {
        Ok(if &self.id == id { Some(self) } else { None })
    }

    #[inline]
    fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s Self>, Self::Error>
    where
        Self: 's + justact::Identifiable,
    {
        Ok(Some(self).into_iter())
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { Ok(1) }
}
