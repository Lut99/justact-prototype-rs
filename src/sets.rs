//  SETS.rs
//    by Lut99
//
//  Created:
//    13 Jan 2025, 15:26:24
//  Last edited:
//    26 Jan 2025, 17:38:44
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the four sets.
//

use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use thiserror::Error;

use crate::wire::{Action, Agreement, Message};

mod justact {
    pub use ::justact::auxillary::{Actored, Authored, Identifiable};
    pub use ::justact::collections::Selector;
    pub use ::justact::collections::map::{Map, MapAsync};
    pub use ::justact::collections::set::{Set, SetSync};
    pub use ::justact::times::{Times, TimesSync};
}


/***** TYPE ALIASES *****/
/// Defines the set of enacted actions.
pub type Actions = MapAsync<Action>;

/// Defines the set of agreements.
pub type Agreements = HashMap<(String, u32), Agreement>;

/// Defines the set of stated messages.
pub type Statements = MapAsync<Arc<Message>>;





/***** ERRORS *****/
/// Errors emitted by the [`MapAsync`]([`view`](MapAsyncView)).
#[derive(Debug, Error)]
pub enum Error<I> {
    /// An agent illegally stated a message out of their control.
    #[error("Agent {agent:?} stated message {message:?} without being its author or knowing it")]
    IllegalStatement { agent: String, message: I },
}





/***** HELPERS *****/
/// Abstracts over both [`Message`]s and [`Action`]s.
trait Agented {
    type AgentId: ?Sized + Eq + Hash;

    fn agent_id(&self) -> &Self::AgentId;
}
impl Agented for Arc<Message> {
    type AgentId = <Arc<Message> as justact::Authored>::AuthorId;

    #[inline]
    fn agent_id(&self) -> &Self::AgentId { <Arc<Message> as justact::Authored>::author_id(self) }
}
impl Agented for Action {
    type AgentId = <Action as justact::Actored>::ActorId;

    #[inline]
    fn agent_id(&self) -> &Self::AgentId { <Action as justact::Actored>::actor_id(self) }
}





/***** LIBRARY *****/
/// Defines a _synchronous_ set for keeping track of the (current) time.
pub struct Times {
    /// The current time, if any.
    current: Option<u64>,
    /// The set of all known times.
    times:   HashSet<u64>,
}
impl Times {
    /// Creates a new, empty Times.
    ///
    /// # Returns
    /// A completely empty Times ready to be used by agents.
    #[inline]
    pub fn new() -> Self { Self { current: None, times: HashSet::new() } }
}
impl justact::Set<u64> for Times {
    type Error = Infallible;

    #[inline]
    fn get(&self, id: &u64) -> Result<Option<&u64>, Self::Error> { Ok(self.times.get(id)) }

    #[inline]
    fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s u64>, Self::Error>
    where
        u128: 's,
    {
        Ok(self.times.iter())
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { Ok(self.times.len()) }
}
impl justact::SetSync<u64> for Times {
    #[inline]
    fn add(&mut self, elem: u64) -> Result<bool, Self::Error> { Ok(self.times.insert(elem)) }
}
impl justact::Times for Times {
    type Subset = Option<u64>;
    type Timestamp = u64;

    #[inline]
    fn current(&self) -> Result<Self::Subset, Self::Error> { Ok(self.current) }
}
impl justact::TimesSync for Times {
    #[inline]
    fn add_current(&mut self, timestamp: Self::Timestamp) -> Result<bool, Self::Error> {
        // Always add to the set
        <Self as justact::SetSync<u64>>::add(self, timestamp)?;

        // Then add it as the current timestamp, but only by checking if it exists already
        let existed: bool = if let Some(current) = self.current { current == timestamp } else { false };
        self.current = Some(timestamp);
        Ok(existed)
    }
}



/// A generic _asynchronous set_, which offers each agent a unique view to it.
pub struct MapAsync<E>
where
    E: justact::Identifiable,
    E::Id: ToOwned,
{
    /// A map of agents to what they see.
    views: HashMap<String, HashMap<<E::Id as ToOwned>::Owned, E>>,
}
impl<E> MapAsync<E>
where
    E: justact::Identifiable,
    E::Id: ToOwned,
{
    /// Creates a new, empty MapAsync.
    ///
    /// # Returns
    /// A completely empty MapAsync ready to be used by agents.
    #[inline]
    pub fn new() -> Self { Self { views: HashMap::new() } }

    /// Registers a new agent.
    ///
    /// This will essentially create a new view for it. Be aware that the agent won't start to
    /// receive messages until it is registered.
    ///
    /// If the agent already existed, nothing happens.
    ///
    /// # Arguments
    /// - `id`: The ID of the agent to register.
    ///
    /// # Returns
    /// True if an agent already existed, or false otherwise.
    pub fn register(&mut self, id: impl Into<String>) -> bool {
        let id: String = id.into();
        let exists: bool = self.views.contains_key(&id);
        if !exists {
            self.views.insert(id, HashMap::new());
        }
        exists
    }

    /// Returns a local view for a particular agent.
    ///
    /// # Arguments
    /// - `id`: The identifier of the agent to return the view for.
    ///
    /// # Returns
    /// A new [`MapAsyncView`] scoped to the agent with the given `id`.
    ///
    /// # Panics
    /// This function will panic if no agent with ID `id` is [registered](AsyncMap::register()).
    #[inline]
    pub fn scope<'s, 'i>(&'s mut self, id: &'i str) -> MapAsyncView<'s, 'i, E> { MapAsyncView::new(self, id) }
}

/// Defines the view of a specific agent on an [`AsyncMap`].
pub struct MapAsyncView<'s, 'i, E>
where
    E: justact::Identifiable,
    E::Id: ToOwned,
{
    /// The parent view.
    parent: &'s mut MapAsync<E>,
    /// The identifier of the agent we're scoping to.
    pub(crate) id: &'i str,
}
impl<'s, 'i, E> MapAsyncView<'s, 'i, E>
where
    E: justact::Identifiable,
    E::Id: ToOwned,
{
    /// Constructor for the MapAsyncView.
    ///
    /// # Arguments
    /// - `parent`: The parent [`MapAsync`] to update when we are destroyed.
    /// - `id`: The identifier of the agent who's view we are representing.
    ///
    /// # Returns
    /// A new MapAsyncView ready to show an agent what's it all about.
    fn new(parent: &'s mut MapAsync<E>, id: &'i str) -> Self { Self { parent, id } }
}
impl<'s, 'i, E> justact::Map<E> for MapAsyncView<'s, 'i, E>
where
    E: justact::Identifiable,
    E::Id: ToOwned,
    <E::Id as ToOwned>::Owned: 'static + Debug + Eq + Hash,
{
    type Error = Error<<E::Id as ToOwned>::Owned>;

    fn get(&self, id: &<E as justact::Identifiable>::Id) -> Result<Option<&E>, Self::Error>
    where
        E: justact::Identifiable,
    {
        Ok(self.parent.views.get(self.id).unwrap_or_else(|| panic!("Cannot operate view for unregistered agent {:?}", self.id)).get(id))
    }

    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a E>, Self::Error>
    where
        E: 'a + justact::Identifiable,
    {
        Ok(self.parent.views.get(self.id).unwrap_or_else(|| panic!("Cannot operate view for unregistered agent {:?}", self.id)).values())
    }

    fn len(&self) -> Result<usize, Self::Error> {
        Ok(self.parent.views.get(self.id).unwrap_or_else(|| panic!("Cannot operate view for unregistered agent {:?}", self.id)).len())
    }
}
impl<'s, 'i, E> justact::MapAsync<str, E> for MapAsyncView<'s, 'i, E>
where
    E: Clone + justact::Identifiable + Agented<AgentId = str>,
    E::Id: ToOwned,
    <E::Id as ToOwned>::Owned: 'static + Debug + Eq + Hash,
{
    #[inline]
    fn add(&mut self, selector: justact::Selector<&str>, elem: E) -> Result<(), Self::Error>
    where
        E: justact::Identifiable,
    {
        // Check if this agent may publish an element with associated author/actor
        // This is OK if:
        //  - They are the author/actor; or
        //  - This message has already been stated by another agent and this agent knows it
        if elem.agent_id() != self.id
            && self
                .parent
                .views
                .get(self.id)
                .unwrap_or_else(|| panic!("Cannot operate view for unregistered agent {:?}", self.id))
                .values()
                .find(|e| elem.id() == e.id())
                .is_none()
        {
            return Err(Error::IllegalStatement { agent: self.id.into(), message: elem.id().to_owned() });
        }

        // Then add the message to the selected agent's view
        // NOTE: Efficiency should be OK despite the clones everywhere, as we assume that messages
        //       are `Arc`'d in our prototype.
        match selector {
            justact::Selector::Agent(id) => {
                self.parent
                    .views
                    .get_mut(id)
                    .unwrap_or_else(|| panic!("Cannot operate view for unregistered agent {id:?}"))
                    .insert(elem.id().to_owned(), elem);
                Ok(())
            },
            justact::Selector::All => {
                let id = elem.id();
                for view in self.parent.views.values_mut() {
                    view.insert(id.to_owned(), elem.clone());
                }
                Ok(())
            },
        }
    }
}
