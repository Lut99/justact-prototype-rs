//  ST ANTONIUS.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 17:45:04
//  Last edited:
//    23 Jan 2025, 12:03:14
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
use justact::collections::map::{InfallibleMapSync as _, Map, MapAsync};
use justact::collections::set::InfallibleSet as _;
use justact::messages::{ConstructableMessage, MessageSet};
use justact::times::Times;
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};

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
    Section5_4_1(State5_4_1),
    Section5_4_2(State5_4_2),
}
impl State {
    /// Forces interpretation as a section 5.4.1 state.
    ///
    /// # Returns
    /// A [`State5_4_1`] describing the state for the first example.
    ///
    /// # Panics
    /// This function panics if this is not for the first state.
    #[inline]
    fn section5_4_1(self) -> State5_4_1 {
        if let Self::Section5_4_1(state) = self { state } else { panic!("Cannot unwrap a non-`State::Section5_4_1` as one") }
    }

    /// Forces interpretation as a section 5.4.2 state.
    ///
    /// # Returns
    /// A [`State5_4_2`] describing the state for the first example.
    ///
    /// # Panics
    /// This function panics if this is not for the first state.
    #[inline]
    fn section5_4_2(self) -> State5_4_2 {
        if let Self::Section5_4_2(state) = self { state } else { panic!("Cannot unwrap a non-`State::Section5_4_2` as one") }
    }
}

/// The St. Antonius' state throughout section 5.4.1.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum State5_4_1 {
    /// We're trying to publish `(st-antonius 1)`, i.e., publishing our dataset.
    PublishDataset,
    /// We're trying to publish our to-be-enacted message `(st-antonius 2)`, i.e., doing Amy's
    /// task.
    ExecuteAmysTask,
    /// We're trying to enact.
    EnactExecuteAmysTask,
    /// We're going to permit Amy to download.
    AuthoriseDownload,
}

/// The St. Antonius' state throughout section 5.4.2.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum State5_4_2 {
    /// We're trying to publish `(st-antonius 1)`, i.e., publishing our dataset.
    PublishDataset,
    /// We've observed Bob's workflow and we want to execute parts of it.
    ExecuteBobsTask,
    /// We want to justify our part in Bob's workflow.
    EnactExecuteBobsTask,
}





/***** LIBRARY *****/
/// The `st-antonius`-agent from section 5.4.1.
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
    pub fn new(script: Script, handle: &StoreHandle) -> Self {
        Self {
            script,
            state: match script {
                Script::Section5_4_1 => State::Section5_4_1(State5_4_1::PublishDataset),
                Script::Section5_4_2 => State::Section5_4_2(State5_4_2::PublishDataset),
            },
            handle: handle.scope(ID),
        }
    }
}
impl Identifiable for StAntonius {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Agent<(String, u32), (String, u32), str, u64> for StAntonius {
    type Error = Error;

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
            Script::Section5_4_1 => {
                // A little state machine with three states:
                match self.state.section5_4_1() {
                    State5_4_1::PublishDataset => {
                        // The St. Antonius publishes their authorization only after Amy has published
                        let target_id: (String, u32) = (super::amy::ID.into(), 1);
                        if view.stated.contains_key(&target_id).cast()? {
                            // Publish ours
                            view.stated.add(Selector::All, create_message(1, self.id(), include_str!("../slick/st-antonius_1.slick"))).cast()?;

                            // ...and mirror the effect on the data plane
                            self.handle
                                .write(
                                    ((self.id().into(), "patients-2024".into()), "patients".into()),
                                    b"billy bob jones\ncharlie brown\nanakin skywalker",
                                )
                                .cast()?;

                            // Done, move to the next state
                            self.state = State::Section5_4_1(State5_4_1::ExecuteAmysTask);
                        }
                        Ok(Poll::Pending)
                    },

                    State5_4_1::ExecuteAmysTask => {
                        // The St. Antonius publishes the fact they've done work sometime after surf published
                        let target_id: (String, u32) = (super::surf::ID.into(), 1);
                        if view.stated.contains_key(&target_id).cast()? {
                            // Publish ours
                            view.stated.add(Selector::All, create_message(2, self.id(), include_str!("../slick/st-antonius_2.slick"))).cast()?;
                            self.state = State::Section5_4_1(State5_4_1::EnactExecuteAmysTask);
                        }
                        Ok(Poll::Pending)
                    },

                    State5_4_1::EnactExecuteAmysTask => {
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
                        for msg in
                            [(super::amy::ID.into(), 1), (super::amdex::ID.into(), 1), (super::st_antonius::ID.into(), 1), (self.id().into(), 2)]
                        {
                            match view.stated.get(&msg).cast()? {
                                Some(msg) => {
                                    just.add(msg.clone());
                                },
                                None => return Ok(Poll::Pending),
                            }
                        }

                        // Now we're confident all messages are there, too; enact!
                        view.enacted.add(Selector::All, create_action(1, self.id(), agree.clone(), just)).cast()?;

                        // Then update the data plane
                        self.handle.read(&((super::amdex::ID.into(), "utils".into()), "entry-count".into())).cast()?;
                        let patients: Option<Vec<u8>> = self.handle.read(&((self.id().into(), "patients-2024".into()), "patients".into())).cast()?;
                        self.handle
                            .write(
                                ((super::amy::ID.into(), "count-patients".into()), "num-patients".into()),
                                patients.map(|p| String::from_utf8_lossy(&p).lines().count()).unwrap_or(0).to_string().as_bytes(),
                            )
                            .cast()?;

                        // Done
                        self.state = State::Section5_4_1(State5_4_1::AuthoriseDownload);
                        Ok(Poll::Pending)
                    },

                    State5_4_1::AuthoriseDownload => {
                        // Wait for Amy's message wanting to do the download appears
                        let target_id: (String, u32) = (super::amy::ID.into(), 2);
                        if view.stated.contains_key(&target_id).cast()? {
                            // It's there, publish our auth
                            view.stated.add(Selector::All, create_message(3, self.id(), include_str!("../slick/st-antonius_3.slick"))).cast()?;
                            Ok(Poll::Ready(()))
                        } else {
                            Ok(Poll::Pending)
                        }
                    },
                }
            },

            Script::Section5_4_2 => match self.state.section5_4_2() {
                State5_4_2::PublishDataset => {
                    // The St. Antonius publishes their dataset at the start, cuz why not
                    view.stated.add(Selector::All, create_message(1, self.id(), include_str!("../slick/st-antonius_1.slick"))).cast()?;

                    // ...and mirror the effect on the data plane
                    self.handle
                        .write(((self.id().into(), "patients-2024".into()), "patients".into()), b"billy bob jones\ncharlie brown\nanakin skywalker")
                        .cast()?;

                    // Done, move to the next state
                    self.state = State::Section5_4_2(State5_4_2::ExecuteBobsTask);
                    Ok(Poll::Pending)
                },

                State5_4_2::ExecuteBobsTask => {
                    // After observing Bob's message, St. Antonius decides (and synchronizes with
                    // the others) they can do step 3. So they do ONCE the data is available.
                    let target_id: (String, u32) = (super::bob::ID.into(), 1);
                    let entry_count_id: ((String, String), String) = ((super::amdex::ID.into(), "utils".into()), "entry-count".into());
                    let consented_id: ((String, String), String) = ((super::bob::ID.into(), "step2".into()), "consented".into());
                    if view.stated.contains_key(&target_id).cast()? && self.handle.exists(&entry_count_id) && self.handle.exists(&consented_id) {
                        // Publish ours
                        view.stated.add(Selector::All, create_message(4, self.id(), include_str!("../slick/st-antonius_4.slick"))).cast()?;

                        // Do the required data accesses
                        let _ = self.handle.read(&entry_count_id).cast()?;
                        let consented = self
                            .handle
                            .read(&consented_id)
                            .cast()?
                            .unwrap_or_else(|| panic!("Failed to get data contents even though we've checked it exists"));
                        self.handle
                            .write(
                                ((super::bob::ID.into(), "step3".into()), "num-consented".into()),
                                String::from_utf8_lossy(&consented).split('\n').count().to_string().as_bytes(),
                            )
                            .cast()?;

                        // Move to the next state
                        self.state = State::Section5_4_2(State5_4_2::EnactExecuteBobsTask);
                    }
                    Ok(Poll::Pending)
                },

                State5_4_2::EnactExecuteBobsTask => {
                    // Let's first wait until the consortium had its chance to publish the agreement/times
                    let agree_id: (String, u32) = (super::consortium::ID.into(), 1);
                    let agree: &Agreement<_, _> = match view.agreed.get(&agree_id).cast()? {
                        Some(agree) => agree,
                        None => return Ok(Poll::Pending),
                    };
                    if !view.times.current().cast()?.contains(&agree.at) {
                        return Ok(Poll::Pending);
                    }

                    // The target agreement is valid; check the required messages!
                    // NOTE: We will only agree once all agents stated they have/can execute it.
                    // Otherwise, our justification will fail, because Bob's message states that
                    // task 4 has been executed (which hasn't been without `amdex`'s involvement).
                    let mut just: MessageSet<SM> = MessageSet::new();
                    for msg in [
                        (super::amdex::ID.into(), 1),
                        (super::bob::ID.into(), 1),
                        (super::st_antonius::ID.into(), 1),
                        (super::st_antonius::ID.into(), 4),
                        (super::surf::ID.into(), 2),
                    ] {
                        match view.stated.get(&msg).cast()? {
                            Some(msg) => {
                                just.add(msg.clone());
                            },
                            None => return Ok(Poll::Pending),
                        }
                    }

                    // We are confident everything we need is there; enact!
                    view.enacted.add(Selector::All, create_action(2, self.id(), agree.clone(), just)).cast()?;

                    // Done!
                    Ok(Poll::Ready(()))
                },
            },
        }
    }
}
