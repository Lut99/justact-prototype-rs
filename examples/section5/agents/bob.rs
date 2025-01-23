//  BOB.rs
//    by Lut99
//
//  Created:
//    22 Jan 2025, 11:04:07
//  Last edited:
//    23 Jan 2025, 13:19:37
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
pub const ID: &'static str = "bob";





/***** HELPERS *****/
/// The St. Antonius' state throughout section 5.4.2.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum State5_4_2 {
    /// We're trying to publish Bob's workflow, together with the claims task 1 / task 4 (will) be
    /// executed.
    PublishWorkflow,
    /// We're going to enact our work.
    EnactWorkflow,
}





/***** LIBRARY *****/
/// The `bob`-agent from section 5.4.1.
pub struct Bob {
    script: Script,
    state:  State5_4_2,
    handle: ScopedStoreHandle,
}
impl Bob {
    /// Constructor for the `bob` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what Amy will do.
    /// - `handle`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Bob agent.
    #[inline]
    #[allow(unused)]
    pub fn new(script: Script, handle: &StoreHandle) -> Self { Self { script, state: State5_4_2::PublishWorkflow, handle: handle.scope(ID) } }
}
impl Identifiable for Bob {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, u32), str, u64> for Bob {
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
        SA: ConstructableAction<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        // Decide which script to execute
        match self.script {
            // Bob doesn't participate in the first example.
            Script::Section5_4_1 => unreachable!(),

            Script::Section5_4_2 => match self.state {
                State5_4_2::PublishWorkflow => {
                    // Bob publishes their statement like, right away, even though he can't deliver on
                    // executing step 4 yet.
                    view.stated.add(Selector::All, create_message(1, self.id(), include_str!("../slick/bob_1.slick"))).cast()?;

                    // Let's write the result of the first step. Bob can do that already.
                    self.handle
                        .write(((self.id().into(), "step1".into()), "filter-consented".into()), b"code_that_actually_filters_consent_wowie();")
                        .cast()?;

                    // Done, move to the next state
                    self.state = State5_4_2::EnactWorkflow;
                    Ok(Poll::Pending)
                },

                State5_4_2::EnactWorkflow => {
                    // First, wait until the agreement is available and applicable
                    let agree_id: (String, u32) = (super::consortium::ID.into(), 1);
                    let agree: &Agreement<_, _> = match view.agreed.get(&agree_id).cast()? {
                        Some(agree) => agree,
                        None => return Ok(Poll::Pending),
                    };
                    if !view.times.current().cast()?.contains(&agree.at) {
                        return Ok(Poll::Pending);
                    }

                    // Then wait for the justification to become stated.
                    let mut just: MessageSet<SM> = MessageSet::new();
                    for msg in [
                        (super::amdex::ID.into(), 1),
                        (self.id().into(), 1),
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

                    // Now enact our work
                    view.enacted.add(Selector::All, create_action(1, self.id(), agree.clone(), just)).cast()?;

                    // Do the result of task 4
                    let _ = self.handle.read(&((self.id().into(), "step3".into()), "num-consented".into())).cast()?;

                    // Done!
                    Ok(Poll::Ready(()))
                },
            },

            // Neither does Bob participate in the fourth
            Script::Section5_4_4 => unreachable!(),
        }
    }
}
