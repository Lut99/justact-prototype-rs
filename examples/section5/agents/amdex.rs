//  AMDEX.rs
//    by Lut99
//
//  Created:
//    15 Jan 2025, 15:22:02
//  Last edited:
//    23 Jan 2025, 15:11:13
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
use justact::collections::Selector;
use justact::collections::map::{Map, MapAsync};
use justact::messages::ConstructableMessage;
use justact::times::Times;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};

use super::{Script, create_message};
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "amdex";





/***** LIBRARY *****/
/// The `amdex`-agent from section 5.4.1.
pub struct Amdex {
    script: Script,
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
    pub fn new(script: Script, handle: &StoreHandle) -> Self { Self { script, handle: handle.scope(ID) } }
}
impl Identifiable for Amdex {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, u32), str, u64> for Amdex {
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
            Script::Section5_4_1 | Script::Section5_4_2 => {
                // The AMdEX agent can publish immediately, it doesn't yet need the agreement for just
                // stating.
                let id: (String, u32) = (self.id().into(), 1);
                match view.stated.contains_key(&id) {
                    Ok(true) => Ok(Poll::Ready(())),
                    Ok(false) => {
                        // Push the message
                        view.stated.add(Selector::All, create_message(id.1, id.0, include_str!("../slick/amdex_1.slick"))).cast()?;

                        // Make the "container" available
                        self.handle.write(((self.id().into(), "utils".into()), "entry-count".into()), b"super_clever_code();").cast()?;

                        // Done
                        Ok(Poll::Ready(()))
                    },
                    Err(err) => Err(Error::new(err)),
                }
            },

            // Not involved in section 5.4.4, but present because they will receive a message from the St. Antonius
            Script::Section5_4_4 => Ok(Poll::Ready(())),
        }
    }
}
