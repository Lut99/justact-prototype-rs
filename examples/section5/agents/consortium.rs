//  CONSORTIUM.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:48:35
//  Last edited:
//    22 Jan 2025, 09:29:35
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a consortium [`Synchronizer`] as described in section 5.3
//!   and 5.4 of the JustAct paper \[1\].
//

use std::ops::ControlFlow;

use justact::actions::ConstructableAction;
use justact::actors::{Synchronizer, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::map::{MapAsync, MapSync};
use justact::collections::set::InfallibleSet as _;
use justact::messages::ConstructableMessage;
use justact::times::TimesSync;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};

use super::create_message;
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "consortium";





/***** AUXILLARY *****/
/// Programs the consortium with a certain section's behaviour.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Behaviour {
    /// The first example, that of section 5.4.1.
    Section5_4_1,
}





/***** LIBRARY *****/
/// The `consortium`-agent from section 5.4.1.
pub struct Consortium {
    behaviour: Behaviour,
    _handle:   ScopedStoreHandle,
}
impl Consortium {
    /// Constructor for the consortium.
    ///
    /// # Arguments
    /// - `behaviour`: A [`Behaviour`] describing what the consortium will do.
    /// - `handle`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Consortium agent.
    #[inline]
    pub fn new(behaviour: Behaviour, handle: &StoreHandle) -> Self { Self { behaviour, _handle: handle.scope(ID) } }
}
impl Identifiable for Consortium {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Synchronizer<(String, u32), (String, u32), str, u64> for Consortium {
    type Error = Error;

    #[inline]
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<ControlFlow<()>, Self::Error>
    where
        T: TimesSync<Timestamp = u64>,
        A: MapSync<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        match self.behaviour {
            Behaviour::Section5_4_1 => {
                // When no time is active yet, the consortium agent will initialize the system by bumping
                // it to `1` and making the initial agreement active.
                let current_times = view.times.current().cast()?;
                if !current_times.contains(&1) {
                    // Add the agreement
                    let agree = Agreement { message: create_message(1, self.id(), include_str!("../slick/agreement.slick")), at: 1 };
                    view.agreed.add(agree).cast()?;

                    // Update the timestamp
                    view.times.add_current(1).cast()?;
                }

                // Done, other agents can have a go (as long as the target isn't enacted yet!)
                let target_id: (String, u32) = (super::amy::ID.into(), 1);
                if view.enacted.contains_key(&target_id).cast()? { Ok(ControlFlow::Break(())) } else { Ok(ControlFlow::Continue(())) }
            },
        }
    }
}
