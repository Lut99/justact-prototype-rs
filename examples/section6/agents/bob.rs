//  BOB.rs
//    by Lut99
//
//  Created:
//    22 Jan 2025, 11:04:07
//  Last edited:
//    31 Jan 2025, 16:02:21
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the Bob agent for section 6.3.2.
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

use super::{Script, create_action, create_message};


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "bob";





/***** LIBRARY *****/
/// The `bob`-agent from section 6.3.1.
pub struct Bob {
    handler: EventHandler,
    store:   ScopedStoreHandle,
}
impl Bob {
    /// Constructor for the `bob` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what Bob will do.
    /// - `store`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Bob agent.
    #[inline]
    #[allow(unused)]
    pub fn new(script: Script, store: &StoreHandle) -> Self {
        if !matches!(script, Script::Section6_3_2 | Script::Section6_3_3_ok | Script::Section6_3_3_crash) {
            panic!("Bob only plays a role in sections 6.3.2 and 6.3.3")
        }
        Self { handler: EventHandler::new(), store: store.scope(ID) }
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
    fn poll<T, A, S, E, SM, SA>(&mut self, view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: Times<Timestamp = u64>,
        A: Map<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, char), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        // Encode Bob's event handler script
        self.handler
            .handle_with_store(view, self.store.clone())
            // Bob publishes his workflow right from the start (`bob 1`).
            .on_start(|view| view.stated.add(Recipient::All, create_message(1, ID, include_str!("../slick/bob_1.slick"))))?
            // He can enact his workflow once the partners of it have confirmed their involvement.
            // Specifically, he's looking for confirmation that someone executes steps 2 and 3.
            .on_agreed_and_stated(
                (super::consortium::ID, 1),
                [
                    (ID, 1),
                    (super::st_antonius::ID, 1),
                    (super::st_antonius::ID, 4),
                    (super::surf::ID, 1),
                    (super::surf::ID, 3)
                ],
                |view, agree, just| view.enacted.add(Recipient::All, create_action('a', ID, agree, MessageSet::from_iter(just)))
            )?
            // Once the enactment is there, do step 1.
            .on_enacted((ID, 'a'), |_, _| self.store.write(((ID, "step1"), "filter-consented"), (ID, 'a'), b"code_that_actually_filters_consent_wowie();"))?
            // Then, once the partners have also written their dataset, it's our turn to do step 4.
            .on_datas_created(
                [
                    ((ID, "step1"), "filter-consented"),
                    ((ID, "step2"), "consented"),
                    ((ID, "step3"), "num-consented")
                ],
                |_| -> Result<(), Error> {
                    let _ = self.store.read(((ID, "step3"), "num-consented"), (ID, 'a')).cast()?;
                    Ok(())
                }
            )?
            .finish()
    }
}
