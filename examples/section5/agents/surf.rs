//  SURF.rs
//    by Lut99
//
//  Created:
//    21 Jan 2025, 14:23:12
//  Last edited:
//    23 Jan 2025, 14:50:18
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
use justact::collections::map::{InfallibleMapSync as _, Map, MapAsync};
use justact::collections::set::InfallibleSet as _;
use justact::messages::{ConstructableMessage, MessageSet};
use justact::times::Times;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};

use super::{Script, create_action, create_message};
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "surf";





/***** HELPERS *****/
/// Defines SURF's state for section 5.4.2.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum State5_4_2 {
    /// Going to announce we'll execute it.
    Execute,
    /// Justify the execution, then do it.
    EnactExecute,
}





/***** LIBRARY *****/
/// The `surf`-agent from section 5.4.1 & 5.4.2.
pub struct Surf {
    script: Script,
    state:  State5_4_2,
    handle: ScopedStoreHandle,
}
impl Surf {
    /// Constructor for the `surf` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what the SURF-agent will do.
    /// - `handle`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Surf agent.
    #[inline]
    pub fn new(script: Script, handle: &StoreHandle) -> Self { Self { script, state: State5_4_2::Execute, handle: handle.scope(ID) } }
}
impl Identifiable for Surf {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, u32), str, u64> for Surf {
    type Error = Error;

    #[track_caller]
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: Times<Timestamp = u64>,
        A: Map<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        // Decide which script to execute
        match self.script {
            Script::Section5_4_1 => {
                // SURF publishes that they do Amy's task as soon as it's available.
                let target_id: (String, u32) = (super::amy::ID.into(), 1);
                if view.stated.contains_key(&target_id).cast()? {
                    // Publish ours
                    view.stated.add(Selector::All, create_message(1, self.id(), include_str!("../slick/surf_1.slick"))).cast()?;
                    return Ok(Poll::Ready(()));
                }

                // Else, keep waiting
                Ok(Poll::Pending)
            },

            Script::Section5_4_2 => match self.state {
                State5_4_2::Execute => {
                    // After observing Bob's message, SURF decides (and synchronizes with the others)
                    // they can do step 2. So they do ONCE the required data is available.
                    let target_id: (String, u32) = (super::bob::ID.into(), 1);
                    let filter_consented_id: ((String, String), String) = ((super::bob::ID.into(), "step1".into()), "filter-consented".into());
                    let patients_id: ((String, String), String) = ((super::st_antonius::ID.into(), "patients-2024".into()), "patients".into());
                    if view.stated.contains_key(&target_id).cast()? && self.handle.exists(&filter_consented_id) && self.handle.exists(&patients_id) {
                        // Publish ours
                        view.stated.add(Selector::All, create_message(2, self.id(), include_str!("../slick/surf_2.slick"))).cast()?;

                        // We already execute our work! The justification later merely serves to prove it.
                        let _ = self.handle.read(&filter_consented_id).cast()?;
                        let _ = self.handle.read(&patients_id).cast()?;
                        // Sadly, we'll emulate the execution for now.
                        self.handle
                            .write(((super::bob::ID.into(), "step2".into()), "consented".into()), b"billy bob jones\nanakin skywalker")
                            .cast()?;

                        // Move to the next state
                        self.state = State5_4_2::EnactExecute;
                    }
                    Ok(Poll::Pending)
                },

                State5_4_2::EnactExecute => {
                    // Let's first wait until the consortium had its chance to publish the agreement/times
                    let agree_id: (String, u32) = (super::consortium::ID.into(), 1);
                    let agree: &Agreement<_, _> = match view.agreed.get(&agree_id).cast()? {
                        Some(agree) => agree,
                        None => return Ok(Poll::Pending),
                    };
                    if !view.times.current().cast()?.contains(&agree.at) {
                        return Ok(Poll::Pending);
                    }

                    // The target agreement is valid; check the required messages!
                    // NOTE: We will only agree once all agents stated they have/can execute it.
                    // Otherwise, our justification will fail, because Bob's message states that
                    // task 4 has been executed (which hasn't been without `amdex`'s involvement).
                    let mut just: MessageSet<SM> = MessageSet::new();
                    for msg in [
                        (super::amdex::ID.into(), 1),
                        (super::bob::ID.into(), 1),
                        (super::st_antonius::ID.into(), 1),
                        (super::st_antonius::ID.into(), 4),
                        (super::surf::ID.into(), 2),
                    ] {
                        match view.stated.get(&msg).cast()? {
                            Some(msg) => {
                                just.add(msg.clone());
                            },
                            None => return Ok(Poll::Pending),
                        }
                    }

                    // We are confident everything we need is there; enact!
                    view.enacted.add(Selector::All, create_action(1, self.id(), agree.clone(), just)).cast()?;

                    // Done!
                    Ok(Poll::Ready(()))
                },
            },

            Script::Section5_4_4 => {
                // After observing St. Antonius' statements, SURF decides to read St. Antonius'
                // dataset based on being trusted.

                // First wait on the agreement
                let agree_id: (String, u32) = (super::consortium::ID.into(), 1);
                let agree: &Agreement<_, _> = match view.agreed.get(&agree_id).cast()? {
                    Some(agree) => agree,
                    None => return Ok(Poll::Pending),
                };
                if !view.times.current().cast()?.contains(&agree.at) {
                    return Ok(Poll::Pending);
                }

                // Then wait for the St. Antonius' statements
                let mut just: MessageSet<SM> = MessageSet::new();
                for msg in [(super::st_antonius::ID.into(), 1), (super::st_antonius::ID.into(), 5)] {
                    match view.stated.get(&msg).cast()? {
                        Some(msg) => {
                            just.add(msg.clone());
                        },
                        None => return Ok(Poll::Pending),
                    }
                }

                // OK, now state our own execution...
                let msg: SM = create_message(3, self.id(), include_str!("../slick/surf_3.slick"));
                just.add(msg.clone());
                view.stated.add(Selector::All, msg).cast()?;

                // ...and then enact it!
                view.enacted.add(Selector::All, create_action(2, self.id(), agree.clone(), just)).cast()?;

                // (and model the read)
                let _ = self.handle.read(&((super::st_antonius::ID.into(), "patients-2024".into()), "patients".into())).cast()?;

                // Done :)
                Ok(Poll::Ready(()))
            },
        }
    }
}