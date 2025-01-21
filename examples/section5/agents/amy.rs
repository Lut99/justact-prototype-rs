//  AMY.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 15:11:36
//  Last edited:
//    21 Jan 2025, 09:55:55
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
use justact::collections::map::{Map, MapAsync};
use justact::collections::set::InfallibleSet;
use justact::collections::{Selector, Singleton};
use justact::messages::ConstructableMessage;
use justact::policies::{Extractor, Policy as _};
use justact::times::Times;
use justact_prototype::policy::slick::{Denotation, Extractor as SlickExtractor};
use slick::GroundAtom;
use slick::text::Text;

use super::create_message;
pub use crate::error::Error;
use crate::error::ResultToError as _;





/***** LIBRARY *****/
/// The `amy`-agent from section 5.4.1.
pub struct Amy;
impl Identifiable for Amy {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "amy" }
}
impl Agent<(String, u32), (String, u32), str, u64> for Amy {
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
        // Amy waits until she sees her package of interest pop into existance
        // I.e., she waits until she sees: `(amdex utils) ready.`
        let pkg = GroundAtom::Tuple(vec![
            GroundAtom::Tuple(vec![GroundAtom::Constant(Text::from_str("amdex")), GroundAtom::Constant(Text::from_str("utils"))]),
            GroundAtom::Constant(Text::from_str("ready")),
        ]);
        let mut state: bool = false;
        for msg in view.stated.iter().cast()? {
            let set = Singleton(msg);
            let denot: Denotation = SlickExtractor.extract(&set).cast()?.truths();
            if denot.is_valid() && <Denotation as InfallibleSet<GroundAtom>>::contains(&denot, &pkg) {
                // The message exists (and is valid)! Publish her snippet.
                state = true;
                break;
            }
        }

        // Publish if we found the target message; else keep waiting
        if state {
            // Push the message
            view.stated.add(Selector::All, create_message(1, self.id(), include_str!("../slick/amy_1.slick"))).cast()?;
            Ok(Poll::Ready(()))
        } else {
            // Amy's not done otherwise
            Ok(Poll::Pending)
        }
    }
}
