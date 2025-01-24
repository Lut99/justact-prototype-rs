//  DAN.rs
//    by Lut99
//
//  Created:
//    21 Jan 2025, 09:25:37
//  Last edited:
//    24 Jan 2025, 23:02:13
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the "Disruptor" Dan agent from section 5.4.1 in the
//!   paper.
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
pub const ID: &'static str = "dan";





/***** LIBRARY *****/
/// The `dan`-agent from section 5.4.1.
pub struct Dan {
    script:  Script,
    _handle: ScopedStoreHandle,
}
impl Dan {
    /// Constructor for the `dan` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what Dan will do.
    /// - `handle`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Dan agent.
    #[inline]
    #[allow(unused)]
    pub fn new(script: Script, handle: &StoreHandle) -> Self { Self { script, _handle: handle.scope(ID) } }
}
impl Identifiable for Dan {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "dan" }
}
impl Agent<(String, u32), (String, char), str, u64> for Dan {
    type Error = Error;

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
            Script::Section5_4_1 => {
                // Dan waits for all the agreements and messages that precede him in the paper to be sent first
                let target_agree: (String, u32) = ("consortium".into(), 1);
                let target_msgs: [(String, u32); 3] = [("amdex".into(), 1), ("amy".into(), 1), ("st-antonius".into(), 1)];
                if !view.agreed.contains_key(&target_agree).cast()? {
                    // Keep waiting
                    return Ok(Poll::Pending);
                }
                for msg in &target_msgs {
                    if !view.stated.contains_key(msg).cast()? {
                        // Keep waiting
                        return Ok(Poll::Pending);
                    }
                }

                // Publish Dan's
                view.stated.add(Selector::All, create_message(1, self.id(), include_str!("../slick/dan_1.slick"))).cast()?;
                Ok(Poll::Ready(()))
            },

            // Dan doesn't participate in the second, fourth or fifth example
            Script::Section5_4_2 | Script::Section5_4_4 | Script::Section5_4_5 => unreachable!(),
        }
    }
}
