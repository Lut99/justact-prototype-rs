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

use std::borrow::Borrow;
use std::convert::Infallible;
use std::fmt::{Formatter, Result as FResult};
use std::sync::Arc;

use ::justact::collections::map::InfallibleMap as _;

use crate::codegen::impl_struct_with_custom_derive;
use crate::policy::{PolicyDeserialize, PolicySerialize};
mod justact {
    pub use ::justact::actions::{Action, ConstructableAction};
    pub use ::justact::agreements::Agreement;
    pub use ::justact::auxillary::{Actored, Authored, Identifiable, Timed};
    pub use ::justact::collections::map::Map;
    pub use ::justact::messages::{ConstructableMessage, Message, MessageSet};
}


/***** TYPE ALIASES *****/
/// Fixes what it means to be an agreement.
pub type Agreement<P> = justact::Agreement<Arc<Message<P>>, u64>;





/***** HELPER FUNCTIONS *****/
/// Akin to [`Message::serialize()`] but external because this crate doesn't own [`justact::Agreement`].
pub fn serialize_agreement<P: ?Sized + PolicySerialize + ToOwned>(agree: &Agreement<P>) -> Agreement<str> {
    justact::Agreement { message: Arc::new(agree.message.serialize()), at: agree.at }
}

/// Akin to [`Message::deserialize()`] but external because this crate doesn't own [`justact::Agreement`].
pub fn deserialize_agreement<'a, P: ?Sized + PolicyDeserialize<'a> + ToOwned>(agree: &'a Agreement<str>) -> Result<Agreement<P>, P::Error> {
    Ok(justact::Agreement { message: Arc::new(agree.message.deserialize()?), at: agree.at })
}





/***** LIBRARY *****/
impl_struct_with_custom_derive! {
    #[derive(Clone, Debug, Deserialize, Serialize)]
    /// Implements a [`Action`](justact::Action) in the prototype.
    pub struct Action<P: ?Sized + ToOwned> {
        /// Identifies this action (as an `(author, id)`-pair).
        pub id: (String, char),
        /// The payload of the action.
        pub basis: Agreement<P>,
        /// The full justification.
        pub extra: justact::MessageSet<Arc<Message<P>>>,
    }
}
// Data management
impl<P: ?Sized + PolicySerialize + ToOwned> Action<P> {
    /// Converts this action into one carrying serialized policy instead.
    ///
    /// # Returns
    /// A new Action, but then one over [`str`]ings instead of `P`.
    #[inline]
    pub fn serialize(&self) -> Action<str> {
        Action {
            id:    self.id.clone(),
            basis: serialize_agreement(&self.basis),
            extra: self.extra.iter().map(|a| &**a).map(Message::serialize).map(Arc::new).collect(),
        }
    }
}
impl Action<str> {
    /// Returns a action that has parsed the internal policy.
    ///
    /// # Generics
    /// - `P`: The type of policy to deserialize the action as.
    ///
    /// # Returns
    /// A new Action, but then one over `P` instead of [`str`]ings.
    ///
    /// # Errors
    /// This function can fail if the action contents were not valid for the chosen `P`olicy type.
    #[inline]
    pub fn deserialize<'a, P: ?Sized + PolicyDeserialize<'a> + ToOwned>(&'a self) -> Result<Action<P>, P::Error> {
        Ok(Action {
            id:    self.id.clone(),
            basis: deserialize_agreement(&self.basis)?,
            extra: self.extra.iter().map(|m| Ok(Arc::new(m.deserialize()?))).collect::<Result<justact::MessageSet<Arc<Message<P>>>, P::Error>>()?,
        })
    }
}
// JustAct
impl<P: ?Sized + ToOwned> justact::ConstructableAction for Action<P>
where
    P::Owned: Clone,
{
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
impl<P: ?Sized + ToOwned> justact::Action for Action<P> {
    type Message = Arc<Message<P>>;

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
        todo!()
    }
}
impl<P: ?Sized + ToOwned> justact::Actored for Action<P> {
    type ActorId = str;

    #[inline]
    fn actor_id(&self) -> &Self::ActorId { &self.id.0 }
}
impl<P: ?Sized + ToOwned> justact::Identifiable for Action<P> {
    type Id = (String, char);

    #[inline]
    fn id(&self) -> &Self::Id { &self.id }
}
impl<P: ?Sized + ToOwned> justact::Timed for Action<P> {
    type Timestamp = u64;

    #[inline]
    fn at(&self) -> &Self::Timestamp { &self.basis.at }
}

impl_struct_with_custom_derive! {
    #[derive(Clone, Debug, Deserialize, Serialize)]
    /// Implements a [`Message`](justact::Message) in the prototype.
    pub struct Message<P: ?Sized + ToOwned> {
        /// Identifies this message (as an `(author, id)`-pair).
        pub id:      (String, u32),
        /// The payload of the message.
        pub payload: P::Owned,
    }
}
// Data management
impl<P: ?Sized + PolicySerialize + ToOwned> Message<P> {
    /// Converts this message into one carrying serialized policy instead.
    ///
    /// # Returns
    /// A new Message, but then one over [`str`]ings instead of `P`.
    #[inline]
    pub fn serialize(&self) -> Message<str> { Message { id: self.id.clone(), payload: self.payload.borrow().serialize() } }
}
impl Message<str> {
    /// Returns a message that has parsed the internal policy.
    ///
    /// # Generics
    /// - `P`: The type of policy to deserialize the message as.
    ///
    /// # Returns
    /// A new Message, but then one over `P` instead of [`str`]ings.
    ///
    /// # Errors
    /// This function can fail if the message contents were not valid for the chosen `P`olicy type.
    #[inline]
    pub fn deserialize<'a, P: ?Sized + PolicyDeserialize<'a> + ToOwned>(&'a self) -> Result<Message<P>, P::Error> {
        Ok(Message { id: self.id.clone(), payload: P::deserialize(&self.payload)? })
    }
}
// JustAct
impl<P: ?Sized + ToOwned> justact::Authored for Message<P> {
    type AuthorId = str;

    #[inline]
    fn author_id(&self) -> &Self::AuthorId { &self.id.0 }
}
impl<P: ?Sized + ToOwned> justact::Identifiable for Message<P> {
    type Id = (String, u32);

    #[inline]
    fn id(&self) -> &Self::Id { &self.id }
}
impl<P: ?Sized + ToOwned> justact::ConstructableMessage for Message<P>
where
    P::Owned: Clone,
{
    #[inline]
    fn new(id: <Self::Id as ToOwned>::Owned, author_id: <Self::AuthorId as ToOwned>::Owned, payload: <Self::Payload as ToOwned>::Owned) -> Self
    where
        Self: Sized,
    {
        Self { id: (author_id.to_owned(), id.to_owned().1), payload: payload.to_owned() }
    }
}
impl<P: ?Sized + ToOwned> justact::Message for Message<P> {
    type Payload = P;

    #[inline]
    fn payload(&self) -> &Self::Payload { self.payload.borrow() }
}
impl<P: ?Sized + ToOwned> justact::Map<Self> for Message<P> {
    type Error = Infallible;

    #[inline]
    fn contains_key(&self, id: &<Self as justact::Identifiable>::Id) -> Result<bool, Self::Error> { Ok(&self.id == id) }

    #[inline]
    fn get(&self, id: &<Self as justact::Identifiable>::Id) -> Result<Option<&Self>, Self::Error> {
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
