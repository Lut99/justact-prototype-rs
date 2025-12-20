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
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use ::justact::collections::set::{InfallibleSet as _, InfallibleSetSync as _};

use crate::codegen::impl_struct_with_custom_derive;
use crate::policy::{PolicyDeserialize, PolicyReflect, PolicySerialize};
mod justact {
    pub use ::justact::actions::{Action, ConstructableAction};
    pub use ::justact::auxillary::{Actored, Authored};
    pub use ::justact::collections::set::Set;
    pub use ::justact::messages::{ConstructableMessage, Message, MessageSet};
}


/***** STATICS *****/
/// Keeps track of messages.
static GLOBAL_MESSAGE_COUNTER: Mutex<u32> = Mutex::new(1);
/// Keeps track of actions.
static GLOBAL_ACTION_COUNTER: Mutex<u32> = Mutex::new(1);





/***** HELPERS *****/
/// Convenience method for turning any [`Message`](justact::Message)-like into a [`Message`].
///
/// # Arguments
/// - `msg`: Some abstract message to convert.
///
/// # Returns
/// A [`Message`], wrapped in [`Arc`].
///
/// Note it returns a serialized version, actually, for reasons that relate to where this function
/// is most used.
#[inline]
pub fn into_prototype_message<SM>(msg: &SM) -> Arc<Message<str>>
where
    SM: justact::Message<AuthorId = str>,
    SM::Payload: PolicySerialize,
{
    Arc::new(Message { human_id: msg.human_id().into(), author_id: msg.author_id().into(), payload: msg.payload().serialize() })
}



/// Convenience method for turning any [`Action`](justact::Action)-like into an [`Action`].
///
/// # Arguments
/// - `msg`: Some abstract message to convert.
///
/// # Returns
/// A [`Action`].
///
/// Note it returns a serialized version, actually, for reasons that relate to where this function
/// is most used.
#[inline]
pub fn into_prototype_action<SM, SA>(act: &SA) -> Action<str>
where
    SM: justact::Message<AuthorId = str>,
    SM::Payload: PolicySerialize,
    SA: justact::Action<ActorId = str, Message = SM>,
{
    Action {
        human_id: act.human_id().into(),
        actor_id: act.actor_id().into(),
        basis:    into_prototype_message(act.basis()),
        extra:    act.extra().iter().map(into_prototype_message).collect(),
    }
}





/***** LIBRARY *****/
impl_struct_with_custom_derive! {
    #[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    /// Implements a [`Action`](justact::Action) in the prototype.
    pub struct Action<P: ?Sized + ToOwned> {
        /// SECRET: An identifier for legibility.
        pub human_id: String,
        /// Identifies this action (as an `(author, id)`-pair).
        pub actor_id: String,
        /// The payload of the action.
        pub basis: Arc<Message<P>>,
        /// The full justification.
        pub extra: justact::MessageSet<Arc<Message<P>>>,
    }
}
// Data management
impl<P: ?Sized + PolicySerialize + ToOwned> Action<P>
where
    P::Owned: Eq + Hash,
{
    /// Converts this action into one carrying serialized policy instead.
    ///
    /// # Returns
    /// A new Action, but then one over [`str`]ings instead of `P`.
    #[inline]
    pub fn serialize(&self) -> Action<str> {
        Action {
            human_id: self.human_id.clone(),
            actor_id: self.actor_id.clone(),
            basis:    Arc::new(self.basis.serialize()),
            extra:    self.extra.iter().map(|a| &**a).map(Message::serialize).map(Arc::new).collect(),
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
    pub fn deserialize<'a, P: ?Sized + PolicyDeserialize<'a> + ToOwned>(&'a self) -> Result<Action<P>, P::Error>
    where
        P::Owned: Eq + Hash,
    {
        Ok(Action {
            human_id: self.human_id.clone(),
            actor_id: self.actor_id.clone(),
            basis:    Arc::new(self.basis.deserialize()?),
            extra:    self
                .extra
                .iter()
                .map(|m| Ok(Arc::new(m.deserialize()?)))
                .collect::<Result<justact::MessageSet<Arc<Message<P>>>, P::Error>>()?,
        })
    }
}
// JustAct
impl<P: ?Sized + PolicyReflect + ToOwned> justact::ConstructableAction for Action<P>
where
    P::Owned: Clone + Eq + Hash,
{
    #[inline]
    fn new(actor_id: <Self::ActorId as ToOwned>::Owned, basis: Self::Message, extra: justact::MessageSet<Self::Message>) -> Self
    where
        Self: Sized,
    {
        let value: u32 = {
            let mut lock = GLOBAL_ACTION_COUNTER.lock().unwrap();
            let value = *lock;
            *lock += 1;
            value
        };
        Self { human_id: format!("{actor_id} {value}"), actor_id, basis, extra }
    }
}
impl<P: ?Sized + PolicyReflect + ToOwned> justact::Action for Action<P>
where
    P::Owned: Eq + Hash,
{
    type Message = Arc<Message<P>>;

    #[inline]
    fn basis(&self) -> &Self::Message { &self.basis }

    #[inline]
    fn extra(&self) -> &justact::MessageSet<Self::Message> { &self.extra }

    #[inline]
    fn payload(&self) -> justact::MessageSet<Self::Message> {
        let mut res = justact::MessageSet::from_iter([self.basis.clone()]);
        for msg in self.extra.iter() {
            res.add(msg.clone());
        }
        res.add(Arc::new(Message { human_id: "TEMP".into(), author_id: self.actor_id.clone(), payload: P::reflect_actor(&self.actor_id) }));
        res
    }

    #[inline]
    fn human_id(&self) -> &str { &self.human_id }
}
impl<P: ?Sized + ToOwned> justact::Actored for Action<P> {
    type ActorId = str;

    #[inline]
    fn actor_id(&self) -> &Self::ActorId { &self.actor_id }
}



impl_struct_with_custom_derive! {
    #[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    /// Implements a [`Message`](justact::Message) in the prototype.
    pub struct Message<P: ?Sized + ToOwned> {
        /// SECRET: An identifier for legibility.
        pub human_id: String,
        /// States the author of the message.
        pub author_id:  String,
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
    pub fn serialize(&self) -> Message<str> {
        Message { human_id: self.human_id.clone(), author_id: self.author_id.clone(), payload: self.payload.borrow().serialize() }
    }
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
        Ok(Message { human_id: self.human_id.clone(), author_id: self.author_id.clone(), payload: P::deserialize(&self.payload)? })
    }
}
// JustAct
impl<P: ?Sized + ToOwned> justact::Authored for Message<P> {
    type AuthorId = str;

    #[inline]
    fn author_id(&self) -> &Self::AuthorId { &self.author_id }
}
impl<P: ?Sized + ToOwned> justact::ConstructableMessage for Message<P>
where
    P::Owned: Clone + Eq + Hash,
{
    #[inline]
    fn new(author_id: <Self::AuthorId as ToOwned>::Owned, payload: <Self::Payload as ToOwned>::Owned) -> Self
    where
        Self: Sized,
    {
        let value: u32 = {
            let mut lock = GLOBAL_MESSAGE_COUNTER.lock().unwrap();
            let value = *lock;
            *lock += 1;
            value
        };
        Self { human_id: format!("{author_id} {value}"), author_id: author_id.to_owned(), payload: payload.to_owned() }
    }
}
impl<P: ?Sized + ToOwned> justact::Message for Message<P>
where
    P::Owned: Eq + Hash,
{
    type Payload = P;

    #[inline]
    fn payload(&self) -> &Self::Payload { self.payload.borrow() }

    #[inline]
    fn human_id(&self) -> &str { &self.human_id }
}
impl<P: ?Sized + ToOwned> justact::Set<Self> for Message<P>
where
    P::Owned: PartialEq,
{
    type Error = Infallible;

    #[inline]
    fn get(&self, elem: &Self) -> Result<Option<&Self>, Self::Error> { Ok(if self == elem { Some(self) } else { None }) }

    #[inline]
    fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s Self>, Self::Error>
    where
        Self: 's,
    {
        Ok(Some(self).into_iter())
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { Ok(1) }
}
