//  CONSORTIUM.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:48:35
//  Last edited:
//    23 Jan 2025, 15:18:54
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

use super::{Script, create_message};
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "consortium";





/***** LIBRARY *****/
/// The `consortium`-agent from section 5.4.1.
pub struct Consortium {
    script:  Script,
    _handle: ScopedStoreHandle,
}
impl Consortium {
    /// Constructor for the consortium.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what the consortium will do.
    /// - `handle`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Consortium agent.
    #[inline]
    pub fn new(script: Script, handle: &StoreHandle) -> Self { Self { script, _handle: handle.scope(ID) } }
}
impl Identifiable for Consortium {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Synchronizer<(String, u32), (String, u32), str, u64> for Consortium {
    type Error = Error;

    #[inline]
    #[track_caller]
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<ControlFlow<()>, Self::Error>
    where
        T: TimesSync<Timestamp = u64>,
        A: MapSync<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        match self.script {
            Script::Section5_4_1 | Script::Section5_4_2 | Script::Section5_4_4 => {
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
                let target_ids: &[(String, u32)] = match self.script {
                    Script::Section5_4_1 => &[(super::amy::ID.into(), 1)],
                    Script::Section5_4_2 => &[(super::bob::ID.into(), 1), (super::st_antonius::ID.into(), 2), (super::surf::ID.into(), 1)],
                    Script::Section5_4_4 => &[(super::st_antonius::ID.into(), 3)],
                };
                for id in target_ids {
                    if !view.enacted.contains_key(id).cast()? {
                        return Ok(ControlFlow::Continue(()));
                    }
                }
                Ok(ControlFlow::Break(()))
            },
        }
    }
}
