//  DAN.rs
//    by Lut99
//
//  Created:
//    21 Jan 2025, 09:25:37
//  Last edited:
//    30 Jan 2025, 21:06:07
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the "Disruptor" Dan agent from section 6.3.1 in the
//!   paper.
//

use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Recipient;
use justact::collections::map::{Map, MapAsync};
use justact::messages::ConstructableMessage;
use justact::times::Times;

use super::{Script, create_message};
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
#[allow(unused)]
pub const ID: &'static str = "dan";





/***** LIBRARY *****/
/// The `dan`-agent from section 6.3.1.
pub struct Dan;
impl Dan {
    /// Constructor for the `dan` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what Dan will do.
    ///
    /// # Returns
    /// A new Dan agent.
    #[inline]
    #[allow(unused)]
    pub fn new(script: Script) -> Self {
        if script != Script::Section6_3_1 && script != Script::Section6_3_3 {
            panic!("Dan only plays a role in sections 6.3.1 and 6.3.3")
        }
        Self
    }
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
        // Dan waits for all the agreements and messages that precede him in the paper to be sent first
        let target_agree: (String, u32) = ("consortium".into(), 1);
        let target_msgs: [(String, u32); 3] = [("surf".into(), 1), ("amy".into(), 1), ("st-antonius".into(), 1)];
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
        view.stated.add(Recipient::All, create_message(1, self.id(), include_str!("../slick/dan_1.slick"))).cast()?;
        Ok(Poll::Ready(()))
    }
}
