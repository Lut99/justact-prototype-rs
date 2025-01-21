//  ST ANTONIUS.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 17:45:04
//  Last edited:
//    21 Jan 2025, 09:57:13
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the St. Antonius agent from section 5.4 in the JustAct
//!   paper \[1\].
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

use super::create_message;
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** LIBRARY *****/
/// The `st-antonius`-agent from section 5.4.1.
pub struct StAntonius;
impl Identifiable for StAntonius {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "st-antonius" }
}
impl Agent<(String, u32), (String, u32), str, u64> for StAntonius {
    type Error = Error;

    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: Times<Timestamp = u64>,
        A: Map<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        // The St. Antonius publishes their authorization only after Amy has published
        let target_id: (String, u32) = ("amy".into(), 1);
        if view.stated.contains_key(&target_id).cast()? {
            // Publish ours
            view.stated.add(Selector::All, create_message(1, self.id(), include_str!("../slick/st-antonius_1.slick"))).cast()?;
            return Ok(Poll::Ready(()));
        }

        // Else, keep waiting
        Ok(Poll::Pending)
    }
}
