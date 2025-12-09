//  AMY.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 15:11:36
//  Last edited:
//    31 Jan 2025, 16:23:39
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
use justact::collections::Recipient;
use justact::collections::map::{Map, MapAsync};
use justact::messages::{ConstructableMessage, MessageSet};
use justact::times::Times;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};
pub use justact_prototype::events::Error;
use justact_prototype::events::{EventHandler, ResultToError as _};
use slick::GroundAtom;
use slick::text::Text;

use super::{Script, create_action, create_message};


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "amy";





/***** LIBRARY *****/
/// The `amy`-agent from section 6.3.1.
pub struct Amy {
    script:  Script,
    handler: EventHandler,
    store:   ScopedStoreHandle,
}
impl Amy {
    /// Constructor for the `amy` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what Amy will do.
    /// - `store`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Amy agent.
    #[inline]
    #[track_caller]
    #[allow(unused)]
    pub fn new(script: Script, store: &StoreHandle) -> Self {
        if !matches!(script, Script::Section6_3_1 | Script::Section6_3_3_ok | Script::Section6_3_3_crash) {
            panic!("Amy only plays a role in sections 6.3.1 and 6.3.3")
        }
        Self { script, handler: EventHandler::new(), store: store.scope(ID) }
    }
}
impl Identifiable for Amy {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, char), str, u64> for Amy {
    type Error = Error;

    #[track_caller]
    fn poll<T, A, S, E, SM, SA>(&mut self, view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: Times<Timestamp = u64>,
        A: Map<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, char), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        // In the third example, where Amy crashes, she just dies from the get-go :/
        if self.script == Script::Section6_3_3_crash {
            return Ok(Poll::Ready(()));
        }

        // Prepare some Slick facts to use
        let surf_utils = GroundAtom::Tuple(vec![
            GroundAtom::Tuple(vec![GroundAtom::Constant(Text::from_str(super::surf::ID)), GroundAtom::Constant(Text::from_str("utils"))]),
            GroundAtom::Constant(Text::from_str("executed")),
        ]);

        // Encode Amy's event handler script
        self.handler
            .handle_with_store(view, self.store.clone())
            // In the first scenario, Amy publishes her execution of `entry-count` on the St.
            // Antonius' dataset.
            // She only does that once she knows the package exists. As such, she waits until she
            // sees: `(surf utils) ready.` before she publishes `amy 1`.
            .on_truth(surf_utils, |view| view.stated.add(Recipient::All, create_message(1, ID, include_str!("../slick/amy_1.slick"))))?
            // Then she waits until the St. Antonius has executed her task. Once so, she publishes
            // her intent to download the result (`amy 2`).
            .on_enacted((super::st_antonius::ID, 'b'), |view, _| view.stated.add(Recipient::All, create_message(2, ID, include_str!("../slick/amy_2.slick"))))?
            // Finally, once she's gotten St. Antonius' authorisation to execute `amy 2`, she'll
            // collect the agreement and all statements (except Dan's) and enact it.
            .on_agreed_and_stated(
                (super::consortium::ID, 1),
                [
                    (super::amy::ID, 1),
                    (super::amy::ID, 2),
                    (super::st_antonius::ID, 1),
                    (super::st_antonius::ID, 2),
                    (super::st_antonius::ID, 3),
                    (super::surf::ID, 1),
                ],
                |view, agree, just| -> Result<(), Error> {
                    view.enacted.add(Recipient::All, create_action('a', ID, agree, MessageSet::from_iter(just))).cast()?;
                    self.store.read(((ID, "count-patients"), "num-patients"), (ID, 'a')).cast()?;
                    Ok(())
                })?
            .finish()
    }
}
