//  SURF.rs
//    by Lut99
//
//  Created:
//    21 Jan 2025, 14:23:12
//  Last edited:
//    22 Jan 2025, 09:10:08
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the SURF agent from section 5.4.1 in the paper \[1\].
//

use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Selector;
use justact::collections::map::{Map, MapAsync};
use justact::messages::ConstructableMessage;
use justact::times::Times;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};

use super::create_message;
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "surf";





/***** LIBRARY *****/
/// The `surf`-agent from section 5.4.1.
pub struct Surf {
    _handle: ScopedStoreHandle,
}
impl Surf {
    /// Constructor for the `surf` agent.
    ///
    /// # Arguments
    /// - `handle`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Surf agent.
    #[inline]
    pub fn new(handle: &StoreHandle) -> Self { Self { _handle: handle.scope(ID) } }
}
impl Identifiable for Surf {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, u32), str, u64> for Surf {
    type Error = Error;

    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: Times<Timestamp = u64>,
        A: Map<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        // SURF publishes that they do Amy's task as soon as it's available.
        let target_id: (String, u32) = (super::amy::ID.into(), 1);
        if view.stated.contains_key(&target_id).cast()? {
            // Publish ours
            view.stated.add(Selector::All, create_message(1, self.id(), include_str!("../slick/surf_1.slick"))).cast()?;
            return Ok(Poll::Ready(()));
        }

        // Else, keep waiting
        Ok(Poll::Pending)
    }
}
