//  ST ANTONIUS.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 17:45:04
//  Last edited:
//    30 Jan 2025, 21:00:31
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
use justact::collections::map::{InfallibleMapSync as _, Map, MapAsync};
use justact::collections::set::InfallibleSet;
use justact::collections::{Recipient, Singleton};
use justact::messages::{ConstructableMessage, MessageSet};
use justact::policies::{Extractor as _, Policy as _};
use justact::times::Times;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};
use justact_prototype::policy::slick::{Denotation as SlickDenotation, Extractor as SlickExtractor};
use slick::GroundAtom;
use slick::text::Text;

use super::{Script, create_action, create_message};
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "st-antonius";





/***** HELPERS *****/
/// The overall St. Antonius state.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum State {
    // The St. Antonius ALWAYS starts with the publishing of their dataset.
    PublishDataset,
    /// We're going to justify- and then write our own dataset.
    DoPublish,

    // The rest is section-dependent.
    Section6_3_1(State6_3_1),
    Section6_3_2(State6_3_2),
    Section6_3_4(State6_3_4),
    Section6_3_5,
}

/// The St. Antonius' state throughout section 6.3.1.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum State6_3_1 {
    /// We're trying to publish our to-be-enacted message `(st-antonius 2)`, i.e., doing Amy's
    /// task.
    ExecuteAmysTask,
    /// We're trying to enact.
    EnactExecuteAmysTask,
    /// We're going to permit Amy to download.
    AuthoriseDownload,
}

/// The St. Antonius' state throughout section 6.3.2.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum State6_3_2 {
    /// We've observed Bob's workflow and we want to execute parts of it.
    ExecuteBobsTask,
    /// Bob has created a justification for us doing work. Let's do it!
    DoWork,
}

/// The St. Antonius' state throughout section 6.3.4.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum State6_3_4 {
    /// Publishing the internalized local policy.
    InternalisedLocalPolicy,
    /// Eventually, they _partially_ publish their further policy.
    PatientPolicy,
}





/***** LIBRARY *****/
/// The `st-antonius`-agent from section 6.3.1.
pub struct StAntonius {
    script: Script,
    state:  State,
    handle: ScopedStoreHandle,
}
impl StAntonius {
    /// Constructor for the `st-antonius` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what the St. Antonius agent will do.
    /// - `handle`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new StAntonius agent.
    #[inline]
    pub fn new(script: Script, handle: &StoreHandle) -> Self { Self { script, state: State::PublishDataset, handle: handle.scope(ID) } }
}
impl Identifiable for StAntonius {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, char), str, u64> for StAntonius {
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
        match self.state {
            State::PublishDataset => {
                // GUARD: Don't do this if we already did it (for scenario 3)
                if !view.stated.contains_key(&(self.id().into(), 1)).cast()? {
                    // The St. Antonius publishes their dataset at the start, cuz why not
                    view.stated.add(Recipient::All, create_message(1, self.id(), include_str!("../slick/st-antonius_1.slick"))).cast()?;
                }

                // Done, move to the next state
                self.state = State::DoPublish;
                Ok(Poll::Pending)
            },
            State::DoPublish => {
                // GUARD: Don't do this if we already did it (for scenario 3)
                if !view.enacted.contains_key(&(self.id().into(), 'a')).cast()? {
                    // Once the agreement is there...
                    let agree_id: (String, u32) = (super::consortium::ID.into(), 1);
                    let agree: &Agreement<_, _> = match view.agreed.get(&agree_id).cast()? {
                        Some(agree) => agree,
                        None => return Ok(Poll::Pending),
                    };
                    if !view.times.current().cast()?.contains(&agree.at) {
                        return Ok(Poll::Pending);
                    }

                    // ...we can justify writing to our own variable...
                    view.enacted
                        .add(
                            Recipient::All,
                            create_action('a', self.id(), agree.clone(), MessageSet::from(view.stated.get(&(self.id().into(), 1)).cast()?.cloned())),
                        )
                        .cast()?;

                    // ...and then write it!
                    self.handle
                        .write(((self.id(), "patients-2024"), "patients"), (self.id(), 'a'), b"billy bob jones\ncharlie brown\nanakin skywalker")
                        .cast()?;
                }

                // Done. Up to this point, it was all script-agnostic; but continue appropriately now
                self.state = match self.script {
                    Script::Section6_3_1 => State::Section6_3_1(State6_3_1::ExecuteAmysTask),
                    Script::Section6_3_2 => State::Section6_3_2(State6_3_2::ExecuteBobsTask),
                    Script::Section6_3_3 => unreachable!(),
                    Script::Section6_3_4 => State::Section6_3_4(State6_3_4::InternalisedLocalPolicy),
                    Script::Section6_3_5 => State::Section6_3_5,
                };
                Ok(Poll::Pending)
            },

            State::Section6_3_1(State6_3_1::ExecuteAmysTask) => {
                // The St. Antonius publishes the fact they've done work sometime after surf published
                let target_id: (String, u32) = (super::surf::ID.into(), 2);
                if view.stated.contains_key(&target_id).cast()? {
                    // Publish ours
                    view.stated.add(Recipient::All, create_message(2, self.id(), include_str!("../slick/st-antonius_2.slick"))).cast()?;
                    self.state = State::Section6_3_1(State6_3_1::EnactExecuteAmysTask);
                }
                Ok(Poll::Pending)
            },
            State::Section6_3_1(State6_3_1::EnactExecuteAmysTask) => {
                // Else, the enactment: enact action antonius 2 when the desired agreement exists and its time is current...
                let agree_id: (String, u32) = (super::consortium::ID.into(), 1);
                let agree: &Agreement<_, _> = match view.agreed.get(&agree_id).cast()? {
                    Some(agree) => agree,
                    None => return Ok(Poll::Pending),
                };
                if !view.times.current().cast()?.contains(&agree.at) {
                    return Ok(Poll::Pending);
                }

                // The target agreement is valid; check the messages!
                let mut just: MessageSet<SM> = MessageSet::new();
                for msg in [(super::amy::ID.into(), 1), (super::surf::ID.into(), 1), (super::st_antonius::ID.into(), 1), (self.id().into(), 2)] {
                    match view.stated.get(&msg).cast()? {
                        Some(msg) => {
                            just.add(msg.clone());
                        },
                        None => return Ok(Poll::Pending),
                    }
                }

                // Now we're confident all messages are there, too; enact!
                view.enacted.add(Recipient::All, create_action('b', self.id(), agree.clone(), just)).cast()?;

                // Then update the data plane
                let enact_id: (&str, char) = (self.id(), 'b');
                self.handle.read(((super::surf::ID, "utils"), "entry-count"), enact_id).cast()?;
                let patients: Option<Vec<u8>> = self.handle.read(((self.id(), "patients-2024"), "patients"), enact_id).cast()?;
                self.handle
                    .write(
                        ((super::amy::ID, "count-patients"), "num-patients"),
                        enact_id,
                        patients.map(|p| String::from_utf8_lossy(&p).lines().count()).unwrap_or(0).to_string().as_bytes(),
                    )
                    .cast()?;

                // Done
                self.state = State::Section6_3_1(State6_3_1::AuthoriseDownload);
                Ok(Poll::Pending)
            },
            State::Section6_3_1(State6_3_1::AuthoriseDownload) => {
                // Wait for Amy's message wanting to do the download appears
                let target_id: (String, u32) = (super::amy::ID.into(), 2);
                if view.stated.contains_key(&target_id).cast()? {
                    // It's there, publish our auth
                    view.stated.add(Recipient::All, create_message(3, self.id(), include_str!("../slick/st-antonius_3.slick"))).cast()?;
                    Ok(Poll::Ready(()))
                } else {
                    Ok(Poll::Pending)
                }
            },

            State::Section6_3_2(State6_3_2::ExecuteBobsTask) => {
                // After observing Bob's message, St. Antonius decides (and synchronizes with
                // the others) they can do step 3. So they do ONCE the data is available.
                let target_id: (String, u32) = (super::bob::ID.into(), 1);
                if view.stated.contains_key(&target_id).cast()? {
                    // Publish ours
                    view.stated.add(Recipient::All, create_message(4, self.id(), include_str!("../slick/st-antonius_4.slick"))).cast()?;

                    // Move to the next state
                    self.state = State::Section6_3_2(State6_3_2::DoWork);
                }
                Ok(Poll::Pending)
            },
            State::Section6_3_2(State6_3_2::DoWork) => {
                // We first wait until Bob's enactment has been done
                if !view.enacted.contains_key(&(super::bob::ID.into(), 'a')).cast()? {
                    return Ok(Poll::Pending);
                }

                // Then we wait until our input data is available
                let entry_count_id: ((String, String), String) = ((super::surf::ID.into(), "utils".into()), "entry-count".into());
                let consented_id: ((String, String), String) = ((super::bob::ID.into(), "step2".into()), "consented".into());
                if !self.handle.exists(&entry_count_id) || !self.handle.exists(&consented_id) {
                    return Ok(Poll::Pending);
                }

                // Now we can do our data accesses
                let enact_id: (&str, char) = (super::bob::ID, 'a');
                let _ = self.handle.read(entry_count_id, enact_id).cast()?;
                let consented = self
                    .handle
                    .read(consented_id, enact_id)
                    .cast()?
                    .unwrap_or_else(|| panic!("Failed to get data contents even though we've checked it exists"));
                self.handle
                    .write(
                        ((super::bob::ID, "step3"), "num-consented"),
                        enact_id,
                        String::from_utf8_lossy(&consented).split('\n').count().to_string().as_bytes(),
                    )
                    .cast()?;

                // Done!
                Ok(Poll::Ready(()))
            },

            State::Section6_3_4(State6_3_4::InternalisedLocalPolicy) => {
                // The St. Antonius can just publish this as they please
                view.stated.add(Recipient::All, create_message(5, self.id(), include_str!("../slick/st-antonius_5.slick"))).cast()?;

                // Done, move to the next state
                self.state = State::Section6_3_4(State6_3_4::PatientPolicy);
                Ok(Poll::Pending)
            },
            State::Section6_3_4(State6_3_4::PatientPolicy) => {
                // We now publish our internal policy but ONLY to trusted agents.
                // Which agents are trusted? We'll read that from our previous snippet!
                let st_antonius_5_id: (String, u32) = (self.id().into(), 5);
                let trusted: Vec<String> = <SlickDenotation as InfallibleSet<GroundAtom>>::iter(
                    &SlickExtractor.extract(&Singleton(view.stated.get(&st_antonius_5_id).cast()?.unwrap())).cast()?.truths(),
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

                // Now publish
                let msg: SM = create_message(6, self.id(), include_str!("../slick/st-antonius_6.slick"));
                for trustee in trusted {
                    view.stated.add(Recipient::One(&trustee), msg.clone()).cast()?;
                }

                // Done
                Ok(Poll::Ready(()))
            },

            State::Section6_3_5 => {
                // Wait for the second agreement to be come valid
                let agree: &Agreement<_, _> = match view.agreed.get(&(super::consortium::ID.into(), 2)).cast()? {
                    Some(agree) => agree,
                    None => return Ok(Poll::Pending),
                };
                if !view.times.current().cast()?.contains(&agree.at) {
                    return Ok(Poll::Pending);
                }

                // Publish that we mark it as insensitive
                view.stated.add(Recipient::All, create_message(7, self.id(), include_str!("../slick/st-antonius_7.slick"))).cast()?;

                // Done!
                Ok(Poll::Ready(()))
            },
        }
    }
}
