//  SURF.rs
//    by Lut99
//
//  Created:
//    21 Jan 2025, 14:23:12
//  Last edited:
//    31 Jan 2025, 17:25:43
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the SURF agent from section 6.3.1 in the paper \[1\].
//

use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Recipient;
use justact::collections::map::{InfallibleMapSync as _, Map, MapAsync};
use justact::messages::{ConstructableMessage, MessageSet};
use justact::times::Times;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};
pub use justact_prototype::events::Error;
use justact_prototype::events::{EventHandler, ResultToError as _};

use super::{Script, create_action, create_message};


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "surf";





/***** LIBRARY *****/
/// The `surf`-agent from section 6.3.1 & 6.3.2.
pub struct Surf {
    script:  Script,
    handler: EventHandler,
    store:   ScopedStoreHandle,
}
impl Surf {
    /// Constructor for the `surf` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what the SURF-agent will do.
    /// - `store`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Surf agent.
    #[inline]
    #[allow(unused)]
    pub fn new(script: Script, store: &StoreHandle) -> Self { Self { script, handler: EventHandler::new(), store: store.scope(ID) } }
}
impl Identifiable for Surf {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, char), str, u64> for Surf {
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
        let mut handler = self.handler.handle_with_store(view, self.store.clone());

        /* First & Second & Third examples */
        if matches!(self.script, Script::Section6_3_1 | Script::Section6_3_2 | Script::Section6_3_3_ok | Script::Section6_3_3_crash) {
            handler = handler
                // SURF publishes the existance of their utils package first.
                .on_start(|view| view.stated.add(Recipient::All, create_message(1, ID, include_str!("../slick/surf_1.slick"))))?

                // Then, once it's published, it enacts it and writes the data.
                .on_agreed_and_stated(
                    (super::consortium::ID, 1),
                    [(ID, 1)],
                    |view, agree, just| -> Result<(), Error> {
                        view.enacted.add(Recipient::All, create_action('a', ID, agree, MessageSet::from_iter(just))).cast()?;
                        self.store.write(((ID, "utils"), "entry-count"), (ID, 'a'), b"super_clever_code();").cast()?;
                        Ok(())
                    }
                )?;
        }

        /* First & Third examples */
        if matches!(self.script, Script::Section6_3_1 | Script::Section6_3_3_ok | Script::Section6_3_3_crash) {
            handler = handler
                // Once SURF's package has been created, SURF notes eventually that Amy has a task
                // to execute. They will publish they can do that for Amy; however, the St.
                // Antonius elects themselves instead, so that's that.
                .on_stated((super::amy::ID, 1), |view, _| view.stated.add(Recipient::All, create_message(2, ID, include_str!("../slick/surf_2.slick"))))?;
        }

        /* Second & Third examples */
        if matches!(self.script, Script::Section6_3_2 | Script::Section6_3_3_ok | Script::Section6_3_3_crash) {
            handler = handler
                // In the second example, SURF will suggest to do the second step once Bob
                // publishes his workflow.
                .on_stated((super::bob::ID, 1), |view, _| view.stated.add(Recipient::All, create_message(3, ID, include_str!("../slick/surf_3.slick"))))?
                
                // Note that not just Bob needs to enact this action; SURF needs to as well to
                // justify their own read! (It's not a valid effect, otherwise.)
                .on_agreed_and_stated(
                    (super::consortium::ID, 1),
                    [
                        (super::bob::ID, 1),
                        (super::st_antonius::ID, 1),
                        (super::st_antonius::ID, 4),
                        (ID, 1),
                        (ID, 3)
                    ],
                    |view, agree, just| view.enacted.add(Recipient::All, create_action('c', ID, agree, MessageSet::from_iter(just)))
                )?

                // Eventually, after Bob enacted the action and his first data becomes available,
                // we do ours.
                .on_enacted_and_datas_created(
                    (super::bob::ID, 'a'),
                    [
                        ((super::bob::ID, "step1"), "filter-consented"),
                        ((super::st_antonius::ID, "patients-2024"), "patients"),
                    ],
                    |_, _| -> Result<(), Error> {
                        let enact_id: (&str, char) = (ID, 'c');
                        let _ = self.store.read(((super::bob::ID, "step1"), "filter-consented"), enact_id).cast()?;
                        let _ = self.store.read(((super::st_antonius::ID, "patients-2024"), "patients"), enact_id).cast()?;
                        // Sadly, we'll emulate the execution for now.
                        self.store.write(((super::bob::ID, "step2"), "consented"), enact_id, b"billy bob jones\nanakin skywalker").cast()?;
                        Ok(())
                    }
                )?
        }

        /* Fourth example */
        if self.script == Script::Section6_3_4 {
            handler = handler
                // In this example, SURF will read St. Antonius' dataset based on their blanket
                // authorisation listing them as trusted.
                .on_agreed_and_stated(
                    (super::consortium::ID, 1),
                    [
                        (super::st_antonius::ID, 1),
                        (super::st_antonius::ID, 5),
                    ],
                    |view, agree, mut just| -> Result<(), Error> {
                        // OK, now state our own execution...
                        let msg: SM = create_message(4, ID, include_str!("../slick/surf_4.slick"));
                        just.add(msg.clone());
                        view.stated.add(Recipient::All, msg).cast()?;

                        // ...and then enact it!
                        view.enacted.add(Recipient::All, create_action('b', ID, agree.clone(), MessageSet::from_iter(just))).cast()?;

                        // (and model the read)
                        let _ = self.store.read(((super::st_antonius::ID, "patients-2024"), "patients"), (ID, 'b')).cast()?;

                        // Done
                        Ok(())
                    },
                )?;
        }

        // Done
        handler.finish()
    }
}
