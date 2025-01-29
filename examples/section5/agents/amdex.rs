//  AMDEX.rs
//    by Lut99
//
//  Created:
//    15 Jan 2025, 15:22:02
//  Last edited:
//    29 Jan 2025, 15:47:39
//  Auto updated?
//    Yes
//
//  Description:
//!   Describes the behaviour of the `amdex` agent as introduced in
//!   section 5.4.1 in the paper \[1\].
//

use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Recipient;
use justact::collections::map::{Map, MapAsync};
use justact::collections::set::InfallibleSet as _;
use justact::messages::{ConstructableMessage, MessageSet};
use justact::times::Times;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};

use super::{Script, create_action, create_message};
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "amdex";





/***** HELPERS *****/
/// Describes the possible states the [`Amdex`] agent is in throughout the examples.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum State {
    /// We proclaim we will publish the utils code.
    PublishEntryCount,
    /// We justify the entry count and then do the work.
    DoWork,
}





/***** LIBRARY *****/
/// The `amdex`-agent from section 5.4.1.
pub struct Amdex {
    script: Script,
    state:  State,
    handle: ScopedStoreHandle,
}
impl Amdex {
    /// Constructor for the `amdex` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what Amy will do.
    /// - `handle`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Amdex agent.
    #[inline]
    #[allow(unused)]
    pub fn new(script: Script, handle: &StoreHandle) -> Self { Self { script, state: State::PublishEntryCount, handle: handle.scope(ID) } }
}
impl Identifiable for Amdex {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, char), str, u64> for Amdex {
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
        // Decide which script to execute
        match self.script {
            Script::Section5_4_1 | Script::Section5_4_2 => match self.state {
                State::PublishEntryCount => {
                    // The AMdEX agent can publish immediately, it doesn't yet need the agreement for just
                    // stating.
                    view.stated.add(Recipient::All, create_message(1, self.id(), include_str!("../slick/amdex_1.slick"))).cast()?;
                    self.state = State::DoWork;
                    Ok(Poll::Pending)
                },
                State::DoWork => {
                    // We will now try to justify our work. So, let's first wait for the agreement...
                    let agree_id: (String, u32) = (super::consortium::ID.into(), 1);
                    let agree: &Agreement<_, _> = match view.agreed.get(&agree_id).cast()? {
                        Some(agree) => agree,
                        None => return Ok(Poll::Pending),
                    };
                    if !view.times.current().cast()?.contains(&agree.at) {
                        return Ok(Poll::Pending);
                    }

                    // Then we justify using our own message only.
                    let amdex_1_id: (String, u32) = (self.id().into(), 1);
                    let just: MessageSet<SM> = MessageSet::from(view.stated.get(&amdex_1_id).cast()?.cloned());
                    view.enacted.add(Recipient::All, create_action('a', self.id(), agree.clone(), just)).cast()?;

                    // With that done, make the "container" available
                    self.handle.write(((self.id(), "utils"), "entry-count"), (self.id(), 'a'), b"super_clever_code();").cast()?;

                    // Done!
                    Ok(Poll::Ready(()))
                },
            },

            // Not involved in section 5.4.4, but present because they will receive a message from the St. Antonius
            Script::Section5_4_4 => Ok(Poll::Ready(())),
            // At all not involved in 5.4.5.
            Script::Section5_4_5 => unreachable!(),
        }
    }
}
