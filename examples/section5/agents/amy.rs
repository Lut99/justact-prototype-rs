//  AMY.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 15:11:36
//  Last edited:
//    23 Jan 2025, 13:19:14
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the `amy` agent from section 5.4 in the paper.
//

use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::map::{InfallibleMapSync as _, Map, MapAsync};
use justact::collections::set::InfallibleSet;
use justact::collections::{Selector, Singleton};
use justact::messages::{ConstructableMessage, MessageSet};
use justact::policies::{Extractor, Policy as _};
use justact::times::Times;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};
use justact_prototype::policy::slick::{Denotation, Extractor as SlickExtractor};
use slick::GroundAtom;
use slick::text::Text;

use super::{Script, create_action, create_message};
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "amy";





/***** HELPERS *****/
/// Amy's state throughout section 5.4.1.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum State {
    /// Amy wants to publish the first task, which executes `count-patients`.
    CountPatients,
    /// Amy wants to publish her intended end task, which downloads the result.
    Download,
    /// Amy wants to enact the downloaded statement.
    EnactDownload,
}





/***** LIBRARY *****/
/// The `amy`-agent from section 5.4.1.
pub struct Amy {
    script: Script,
    state:  State,
    handle: ScopedStoreHandle,
}
impl Amy {
    /// Constructor for the `amy` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what Amy will do.
    /// - `handle`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Amy agent.
    #[inline]
    #[allow(unused)]
    pub fn new(script: Script, handle: &StoreHandle) -> Self { Self { script, state: State::CountPatients, handle: handle.scope(ID) } }
}
impl Identifiable for Amy {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, u32), str, u64> for Amy {
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
                match self.state {
                    State::CountPatients => {
                        // Amy waits until she sees her package of interest pop into existance
                        // I.e., she waits until she sees: `(amdex utils) ready.`
                        let pkg = GroundAtom::Tuple(vec![
                            GroundAtom::Tuple(vec![
                                GroundAtom::Constant(Text::from_str(super::amdex::ID)),
                                GroundAtom::Constant(Text::from_str("utils")),
                            ]),
                            GroundAtom::Constant(Text::from_str("ready")),
                        ]);
                        let mut found_requirements: bool = false;
                        for msg in view.stated.iter().cast()? {
                            let set = Singleton(msg);
                            let denot: Denotation = SlickExtractor.extract(&set).cast()?.truths();
                            if denot.is_valid() && <Denotation as InfallibleSet<GroundAtom>>::contains(&denot, &pkg) {
                                // The message exists (and is valid)! Publish her snippet.
                                found_requirements = true;
                                break;
                            }
                        }

                        // Publish if we found the target message; else keep waiting
                        if found_requirements {
                            // Push the message
                            view.stated.add(Selector::All, create_message(1, self.id(), include_str!("../slick/amy_1.slick"))).cast()?;
                            self.state = State::Download;
                        }

                        // We're anyhow going to continue running
                        Ok(Poll::Pending)
                    },

                    State::Download => {
                        // We wait until we see St. Antonius' enacted statement, making the to-be-
                        // downloaded dataset available
                        let target_id: (String, u32) = (super::st_antonius::ID.into(), 1);
                        if view.enacted.contains_key(&target_id).cast()? {
                            // Push the message
                            view.stated.add(Selector::All, create_message(2, self.id(), include_str!("../slick/amy_2.slick"))).cast()?;
                            self.state = State::EnactDownload;
                        }

                        // We're anyhow going to continue running
                        Ok(Poll::Pending)
                    },

                    State::EnactDownload => {
                        // First wait to ensure the required agreement exists
                        let agree_id: (String, u32) = (super::consortium::ID.into(), 1);
                        let agree: &Agreement<_, _> = match view.agreed.get(&agree_id).cast()? {
                            Some(agree) => agree,
                            None => return Ok(Poll::Pending),
                        };
                        if !view.times.current().cast()?.contains(&agree.at) {
                            return Ok(Poll::Pending);
                        }

                        // Now we wait until we have all the required messages
                        let mut just: MessageSet<SM> = MessageSet::new();
                        for msg in [
                            (super::amdex::ID.into(), 1),
                            (super::amy::ID.into(), 1),
                            (super::amy::ID.into(), 2),
                            (super::st_antonius::ID.into(), 1),
                            (super::st_antonius::ID.into(), 2),
                            (super::st_antonius::ID.into(), 3),
                        ] {
                            match view.stated.get(&msg).cast()? {
                                Some(msg) => {
                                    just.add(msg.clone());
                                },
                                None => return Ok(Poll::Pending),
                            }
                        }

                        // We have them all; enact!
                        view.enacted.add(Selector::All, create_action(1, self.id(), agree.clone(), just)).cast()?;

                        // Then update the data plane
                        self.handle.read(&((self.id().into(), "count-patients".into()), "num-patients".into())).cast()?;

                        // Amy's done!
                        Ok(Poll::Ready(()))
                    },
                }
            },

            // Amy doesn't participate in the second nor fourth example
            Script::Section5_4_2 | Script::Section5_4_4 => unreachable!(),
        }
    }
}
