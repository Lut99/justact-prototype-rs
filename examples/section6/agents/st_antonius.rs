//  ST ANTONIUS.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 17:45:04
//  Last edited:
//    31 Jan 2025, 17:35:38
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
use justact::collections::map::{Map, MapAsync};
use justact::collections::set::InfallibleSet;
use justact::collections::{Recipient, Singleton};
use justact::messages::ConstructableMessage;
use justact::policies::{Extractor as _, Policy as _};
use justact::times::Times;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};
pub use justact_prototype::events::Error;
use justact_prototype::events::{EventHandler, ResultToError as _};
use justact_prototype::policy::slick::{Denotation as SlickDenotation, Extractor as SlickExtractor};
use slick::GroundAtom;
use slick::text::Text;

use super::{Script, create_action, create_message};


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "st-antonius";





/***** LIBRARY *****/
/// The `st-antonius`-agent from section 6.3.1.
pub struct StAntonius {
    script:  Script,
    handler: EventHandler,
    store:   ScopedStoreHandle,
}
impl StAntonius {
    /// Constructor for the `st-antonius` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what the St. Antonius agent will do.
    /// - `store`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new StAntonius agent.
    #[inline]
    pub fn new(script: Script, store: &StoreHandle) -> Self { Self { script, handler: EventHandler::new(), store: store.scope(ID) } }
}
impl Identifiable for StAntonius {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, char), str, u64> for StAntonius {
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
        let mut handler = self.handler.handle_with_store(view, self.store.clone())
            // The St. Antonius will always publish they have the `patients` dataset.
            .on_start(|view| -> Result<(), S::Error> {
                view.stated.add(Recipient::All, create_message(1, ID, include_str!("../slick/st-antonius_1.slick")))?;
                // Also do the one of example 4 if we need to
                if self.script == Script::Section6_3_4 {
                    view.stated.add(Recipient::All, create_message(5, ID, include_str!("../slick/st-antonius_5.slick")))?;
                }
                Ok(())
            })?

            // And once they did so, they'll always try to enact- and write it.
            .on_agreed_and_stated(
                (super::consortium::ID, 1),
                [(ID, 1)],
                |view, agree, just| -> Result<(), Error> {
                    // We can justify writing to our own variable...
                    view.enacted.add(Recipient::All, create_action('a', ID, agree, just)).cast()?;
                    // ...and then write it!
                    self.store
                        .write(((ID, "patients-2024"), "patients"), (ID, 'a'), b"billy bob jones\ncharlie brown\nanakin skywalker")
                        .cast()?;
                    Ok(())
                }
            )?;

        /* First & Third examples */
        if matches!(self.script, Script::Section6_3_1 | Script::Section6_3_3_ok | Script::Section6_3_3_crash) {
            handler = handler
                // After Amy has put a task up for grabs, the St. Antonius will do it themselves.
                .on_stated((super::amy::ID, 1), |view, _| view.stated.add(Recipient::All, create_message(2, ID, include_str!("../slick/st-antonius_2.slick"))))?

                // Then the St. Antonius will enact its own statement.
                .on_agreed_and_stated(
                    (super::consortium::ID, 1),
                    [
                        (super::amy::ID, 1),
                        (super::st_antonius::ID, 1),
                        (super::st_antonius::ID, 2),
                        (super::surf::ID, 1),
                    ],
                    |view, agree, just| view.enacted.add(Recipient::All, create_action('b', ID, agree.clone(), just))
                )?
                // Then, after waiting for the input to be available, it updates the data plane.
                .on_enacted_and_datas_created(
                    (ID, 'b'),
                    [
                        ((super::st_antonius::ID, "patients-2024"), "patients"),
                        ((super::surf::ID, "utils"), "entry-count"),
                    ],
                    |_, _| -> Result<(), Error> {
                        let enact_id: (&str, char) = (ID, 'b');
                        let _ = self.store.read(((super::surf::ID, "utils"), "entry-count"), enact_id).cast()?;
                        let patients: Option<Vec<u8>> = self.store.read(((ID, "patients-2024"), "patients"), enact_id).cast()?;
                        self.store
                            .write(
                                ((super::amy::ID, "count-patients"), "num-patients"),
                                enact_id,
                                patients.map(|p| String::from_utf8_lossy(&p).lines().count()).unwrap_or(0).to_string().as_bytes(),
                            )
                            .cast()?;
                        Ok(())
                    },
                )?

                // Eventually, Amy will have published her request to download. Which we authorise.
                .on_stated((super::amy::ID, 2), |view, _| view.stated.add(Recipient::All, create_message(3, ID, include_str!("../slick/st-antonius_3.slick"))))?;
        }

        /* Second & Third examples */
        if matches!(self.script, Script::Section6_3_2 | Script::Section6_3_3_ok | Script::Section6_3_3_crash) {
            handler = handler
                // After Bob has published their workflow, the St. Antonius elects to do task 3,
                // giving SURF authorisation to do task 2 while at it.
                .on_stated((super::bob::ID, 1), |view, _| view.stated.add(Recipient::All, create_message(4, ID, include_str!("../slick/st-antonius_4.slick"))))?

                // Note that not just Bob needs to enact this action; St. Antonius needs to as well
                // to justify their own read! (It's not a valid effect, otherwise.)
                .on_agreed_and_stated(
                    (super::consortium::ID, 1),
                    [
                        (super::bob::ID, 1),
                        (ID, 1),
                        (ID, 4),
                        (super::surf::ID, 1),
                        (super::surf::ID, 3)
                    ],
                    |view, agree, just| view.enacted.add(Recipient::All, create_action('c', ID, agree, just))
                )?

                // Eventually, after Bob enacted the action and SURF's data becomes available,
                // we do ours.
                .on_enacted_and_datas_created(
                    (super::bob::ID, 'a'),
                    [
                        ((super::bob::ID, "step2"), "consented"),
                        ((super::surf::ID, "utils"), "entry-count"),
                    ],
                    |_, _| -> Result<(), Error> {
                        // Now we can do our data accesses
                        let enact_id: (&str, char) = (ID, 'c');
                        let _ = self.store.read(((super::surf::ID, "utils"), "entry-count"), enact_id).cast()?;
                        let consented = self
                            .store
                            .read(((super::bob::ID, "step2"), "consented"), enact_id)
                            .cast()?
                            .unwrap_or_else(|| panic!("Failed to get data contents even though we've checked it exists"));
                        self.store
                            .write(
                                ((super::bob::ID, "step3"), "num-consented"),
                                enact_id,
                                String::from_utf8_lossy(&consented).split('\n').count().to_string().as_bytes(),
                            )
                            .cast()?;
                        Ok(())
                    }
                )?;
        }

        /* Fourth example */
        if self.script == Script::Section6_3_4 {
            handler = handler
                // We will publish our internalised policy at the start.
                // NOTE: See the initial two triggers above. Currently, agents can only use
                //       `on_start()` once.

                // Then, we provide the patient consent, but send that information only to trusted
                // agents. Who is trusted, we'll read from the previous message.
                .on_stated((ID, 5), |view, msg| -> Result<(), Error> {
                    // Collect the trusted agents
                    let trusted: Vec<String> = <SlickDenotation as InfallibleSet<GroundAtom>>::iter(
                        &SlickExtractor.extract(&Singleton(msg)).cast()?.truths(),
                    )
                    .filter_map(|g| match g {
                        GroundAtom::Constant(_) => None,
                        GroundAtom::Tuple(atoms) => {
                            if atoms.len() == 4 {
                                if let GroundAtom::Constant(first) = atoms[0] {
                                    if atoms[1] == GroundAtom::Constant(Text::from_str("is"))
                                        && atoms[2] == GroundAtom::Constant(Text::from_str("highly"))
                                        && atoms[3] == GroundAtom::Constant(Text::from_str("trusted"))
                                    {
                                        Some(format!("{first:?}"))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        },
                    })
                    .collect();

                    // Publish the new message to those agents only
                    let msg: SM = create_message(6, ID, include_str!("../slick/st-antonius_6.slick"));
                    for trustee in trusted {
                        view.stated.add(Recipient::One(&trustee), msg.clone()).cast()?;
                    }

                    // Done
                    Ok(())
                })?;
        }

        /* Fifth example */
        if self.script == Script::Section6_3_5 {
            handler = handler
                // In the final example, we end with publishing some information useful for the
                // second agreement!
                .on_agreed((super::consortium::ID, 2), |view, _| view.stated.add(Recipient::All, create_message(7, ID, include_str!("../slick/st-antonius_7.slick"))))?;
        }

        // Done!
        handler.finish()
    }
}
