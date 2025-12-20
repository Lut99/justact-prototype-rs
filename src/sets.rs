//  SETS.rs
//    by Lut99
//
//  Created:
//    13 Jan 2025, 15:26:24
//  Last edited:
//    29 Jan 2025, 22:05:13
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the four sets.
//

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use thiserror::Error;

use crate::wire::{Action, Message};

mod justact {
    pub use ::justact::auxillary::{Actored, Authored};
    pub use ::justact::collections::Recipient;
    pub use ::justact::collections::set::{Set, SetAsync};
}


/***** TYPE ALIASES *****/
/// Defines the set of enacted actions.
pub type Actions<P> = SetAsync<Action<P>>;

/// Defines the set of agreements.
pub type Agreements<P> = HashSet<Arc<Message<P>>>;

/// Defines the set of stated messages.
pub type Statements<P> = SetAsync<Arc<Message<P>>>;





/***** ERRORS *****/
/// Errors emitted by the [`MapAsync`]([`view`](MapAsyncView)).
#[derive(Debug, Error)]
pub enum Error<M> {
    /// An agent illegally stated a message out of their control.
    #[error("Agent {agent:?} stated message {message:?} without being its author or knowing it")]
    IllegalStatement { agent: String, message: M },
}





/***** HELPERS *****/
/// Abstracts over both [`Message`]s and [`Action`]s.
trait Agented {
    type AgentId: ?Sized + Eq + Hash;

    fn agent_id(&self) -> &Self::AgentId;
}
impl<P: ?Sized + ToOwned> Agented for Arc<Message<P>> {
    type AgentId = <Arc<Message<P>> as justact::Authored>::AuthorId;

    #[inline]
    fn agent_id(&self) -> &Self::AgentId { <Arc<Message<P>> as justact::Authored>::author_id(self) }
}
impl<P: ?Sized + ToOwned> Agented for Action<P> {
    type AgentId = <Action<P> as justact::Actored>::ActorId;

    #[inline]
    fn agent_id(&self) -> &Self::AgentId { <Action<P> as justact::Actored>::actor_id(self) }
}





/***** LIBRARY *****/
/// A generic _asynchronous set_, which offers each agent a unique view to it.
pub struct SetAsync<E> {
    /// A map of agents to what they see.
    views: HashMap<String, HashSet<E>>,
}
impl<E> Default for SetAsync<E> {
    #[inline]
    fn default() -> Self { Self::new() }
}
impl<E> SetAsync<E> {
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
            self.views.insert(id, HashSet::new());
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
    pub fn scope<'s, 'i>(&'s mut self, id: &'i str) -> SetAsyncView<'s, 'i, E> { SetAsyncView::new(self, id) }
}

/// Defines the view of a specific agent on an [`AsyncMap`].
pub struct SetAsyncView<'s, 'i, E> {
    /// The parent view.
    parent: &'s mut SetAsync<E>,
    /// The identifier of the agent we're scoping to.
    pub(crate) id: &'i str,
}
impl<'s, 'i, E> SetAsyncView<'s, 'i, E> {
    /// Constructor for the MapAsyncView.
    ///
    /// # Arguments
    /// - `parent`: The parent [`MapAsync`] to update when we are destroyed.
    /// - `id`: The identifier of the agent who's view we are representing.
    ///
    /// # Returns
    /// A new MapAsyncView ready to show an agent what's it all about.
    fn new(parent: &'s mut SetAsync<E>, id: &'i str) -> Self { Self { parent, id } }
}
impl<'s, 'i, E: 'static + Debug + Eq + Hash + Send> justact::Set<E> for SetAsyncView<'s, 'i, E> {
    type Error = Error<E>;

    fn get(&self, elem: &E) -> Result<Option<&E>, Self::Error> {
        Ok(self.parent.views.get(self.id).unwrap_or_else(|| panic!("Cannot operate view for unregistered agent {:?}", self.id)).get(elem))
    }

    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a E>, Self::Error>
    where
        E: 'a,
    {
        Ok(self.parent.views.get(self.id).unwrap_or_else(|| panic!("Cannot operate view for unregistered agent {:?}", self.id)).iter())
    }

    fn len(&self) -> Result<usize, Self::Error> {
        Ok(self.parent.views.get(self.id).unwrap_or_else(|| panic!("Cannot operate view for unregistered agent {:?}", self.id)).len())
    }
}
impl<'s, 'i, E: 'static + Agented<AgentId = str> + Clone + Debug + Eq + Hash + Send> justact::SetAsync<str, E> for SetAsyncView<'s, 'i, E> {
    #[inline]
    fn add(&mut self, selector: justact::Recipient<String>, elem: E) -> Result<(), Self::Error> {
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
                .iter()
                .find(|e| &elem == *e)
                .is_none()
        {
            return Err(Error::IllegalStatement { agent: self.id.into(), message: elem });
        }

        // Then add the message to the selected agent's view
        // NOTE: Efficiency should be OK despite the clones everywhere, as we assume that messages
        //       are `Arc`'d in our prototype.
        match selector {
            justact::Recipient::All => {
                for view in self.parent.views.values_mut() {
                    view.insert(elem.clone());
                }
                Ok(())
            },
            justact::Recipient::One(id) => {
                self.parent.views.get_mut(&id).unwrap_or_else(|| panic!("Cannot operate view for unregistered agent {id:?}")).insert(elem);
                Ok(())
            },
        }
    }
}
