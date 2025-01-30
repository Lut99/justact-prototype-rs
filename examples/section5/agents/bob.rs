//  BOB.rs
//    by Lut99
//
//  Created:
//    22 Jan 2025, 11:04:07
//  Last edited:
//    30 Jan 2025, 18:57:47
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the Bob agent for section 5.4.2.
//

use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::map::{InfallibleMapSync as _, Map, MapAsync};
use justact::collections::set::InfallibleSet;
use justact::collections::{Recipient, Singleton};
use justact::messages::{ConstructableMessage, MessageSet};
use justact::policies::{Denotation, Extractor as _, Policy as _};
use justact::times::Times;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};
use justact_prototype::policy::slick::{Denotation as SlickDenotation, Extractor as SlickExtractor};
use slick::GroundAtom;
use slick::text::Text;

use super::{Script, create_action, create_message};
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "bob";





/***** HELPERS *****/
/// The St. Antonius' state throughout section 5.4.2.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum State {
    /// We're trying to publish Bob's workflow, together with the claims task 1 / task 4 (will) be
    /// executed.
    PublishWorkflow,
    /// We're going to enact our work.
    EnactWorkflow,
    /// We do the promised work of the first step
    DoStep1,
    /// We do the promised work of the fourth step
    DoStep4,
}





/***** LIBRARY *****/
/// The `bob`-agent from section 5.4.1.
pub struct Bob {
    state:  State,
    handle: ScopedStoreHandle,
}
impl Bob {
    /// Constructor for the `bob` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what Bob will do.
    /// - `handle`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Bob agent.
    #[inline]
    #[allow(unused)]
    pub fn new(script: Script, handle: &StoreHandle) -> Self {
        if script != Script::Section5_4_2 && script != Script::Section5_4_3 {
            panic!("Bob only plays a role in sections 5.4.2 and 5.4.3")
        }
        Self { state: State::PublishWorkflow, handle: handle.scope(ID) }
    }
}
impl Identifiable for Bob {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, char), str, u64> for Bob {
    type Error = Error;

    #[inline]
    #[track_caller]
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: Times<Timestamp = u64>,
        A: Map<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, char), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        match self.state {
            State::PublishWorkflow => {
                // Bob publishes their statement like, right away, even though he can't deliver on
                // executing step 4 yet.
                view.stated.add(Recipient::All, create_message(1, self.id(), include_str!("../slick/bob_1.slick"))).cast()?;

                // Done, move to the next state
                self.state = State::EnactWorkflow;
                Ok(Poll::Pending)
            },

            State::EnactWorkflow => {
                // First, wait until the agreement is available and applicable
                let agree_id: (String, u32) = (super::consortium::ID.into(), 1);
                let agree: &Agreement<_, _> = match view.agreed.get(&agree_id).cast()? {
                    Some(agree) => agree,
                    None => return Ok(Poll::Pending),
                };
                if !view.times.current().cast()?.contains(&agree.at) {
                    return Ok(Poll::Pending);
                }

                // Ensure that our own message and the "code/data" messages are available
                let mut just: MessageSet<SM> = MessageSet::new();
                for msg in [(super::surf::ID.into(), 1), (self.id().into(), 1), (super::st_antonius::ID.into(), 1)] {
                    match view.stated.get(&msg).cast()? {
                        Some(msg) => {
                            just.add(msg.clone());
                        },
                        None => return Ok(Poll::Pending),
                    }
                }

                // Finally, wait for the St. Antonius and SURF to indicate they agree to the execution.
                let surf_execute: GroundAtom = GroundAtom::Tuple(vec![
                    GroundAtom::Tuple(vec![GroundAtom::Constant(Text::from_str("bob")), GroundAtom::Constant(Text::from_str("step2"))]),
                    GroundAtom::Constant(Text::from_str("executed")),
                ]);
                let st_antonius_execute: GroundAtom = GroundAtom::Tuple(vec![
                    GroundAtom::Tuple(vec![GroundAtom::Constant(Text::from_str("bob")), GroundAtom::Constant(Text::from_str("step3"))]),
                    GroundAtom::Constant(Text::from_str("executed")),
                ]);
                for msg in view.stated.iter().cast()? {
                    let truths: SlickDenotation = SlickExtractor.extract(&Singleton(msg)).cast()?.truths();
                    // Check if we found SURF's execute
                    if msg.id().0 == super::surf::ID && truths.truth_of(&surf_execute) == Some(true) {
                        just.add(msg.clone());
                    }
                    // Check if we found the St. Antonius' execute
                    if msg.id().0 == super::st_antonius::ID && truths.truth_of(&st_antonius_execute) == Some(true) {
                        just.add(msg.clone());
                    }
                }
                if just.len().cast()? != 5 {
                    return Ok(Poll::Pending);
                }

                // Now enact our work
                view.enacted.add(Recipient::All, create_action('a', self.id(), agree.clone(), just)).cast()?;

                // Done!
                self.state = State::DoStep1;
                Ok(Poll::Pending)
            },

            State::DoStep1 => {
                // We execute the first task; we can kick that off immediately!
                self.handle
                    .write(((self.id(), "step1"), "filter-consented"), (self.id(), 'a'), b"code_that_actually_filters_consent_wowie();")
                    .cast()?;
                self.state = State::DoStep4;
                Ok(Poll::Pending)
            },

            State::DoStep4 => {
                // At this point, we've already enacted; only wait for the data to become
                // available and GO!
                let task3_result_id: ((String, String), String) = ((self.id().into(), "step3".into()), "num-consented".into());
                if self.handle.exists(&task3_result_id) {
                    // Bob will now do SUPER interesting stuff with this dataset
                    let _ = self.handle.read(task3_result_id, (self.id(), 'a')).cast()?;
                    Ok(Poll::Ready(()))
                } else {
                    Ok(Poll::Pending)
                }
            },
        }
    }
}
