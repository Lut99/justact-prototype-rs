//  ST ANTONIUS.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 17:45:04
//  Last edited:
//    21 Jan 2025, 17:14:46
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

use super::{create_action, create_message};
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "st-antonius";





/***** HELPERS *****/
/// The St. Antonius' state throughout section 5.4.1.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum State {
    /// We're trying to publish `(st-antonius 1)`
    FirstMessage,
    /// We're trying to publish our to-be-enacted message `(st-antonius 2)`
    SecondMessage,
    /// We're trying to enact.
    Enact,
}





/***** LIBRARY *****/
/// The `st-antonius`-agent from section 5.4.1.
pub struct StAntonius {
    state:  State,
    handle: ScopedStoreHandle,
}
impl StAntonius {
    /// Constructor for the `st-antonius` agent.
    ///
    /// # Arguments
    /// - `handle`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new StAntonius agent.
    #[inline]
    pub fn new(handle: &StoreHandle) -> Self { Self { state: State::FirstMessage, handle: handle.scope(ID) } }
}
impl Identifiable for StAntonius {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
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
        // A little state machine with three state:
        match self.state {
            State::FirstMessage => {
                // The St. Antonius publishes their authorization only after Amy has published
                let target_id: (String, u32) = (super::amy::ID.into(), 1);
                if view.stated.contains_key(&target_id).cast()? {
                    // Publish ours
                    self.handle
                        .write(((self.id().into(), "patients-2024".into()), "patients".into()), b"billy bob jones\ncharlie brown\nanakin skywalker")
                        .cast()?;
                    view.stated.add(Selector::All, create_message(1, self.id(), include_str!("../slick/st-antonius_1.slick"))).cast()?;
                    self.state = State::SecondMessage;
                }
                Ok(Poll::Pending)
            },

            State::SecondMessage => {
                // The St. Antonius publishes the fact they've done work sometime after surf published
                let target_id: (String, u32) = (super::surf::ID.into(), 1);
                if view.stated.contains_key(&target_id).cast()? {
                    // Publish ours
                    view.stated.add(Selector::All, create_message(2, self.id(), include_str!("../slick/st-antonius_2.slick"))).cast()?;
                    self.state = State::Enact;
                }
                Ok(Poll::Pending)
            },

            State::Enact => {
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
                for msg in [(super::amy::ID.into(), 1), (super::amdex::ID.into(), 1), (super::st_antonius::ID.into(), 1), (self.id().into(), 2)] {
                    match view.stated.get(&msg).cast()? {
                        Some(msg) => {
                            just.add(msg.clone());
                        },
                        None => return Ok(Poll::Pending),
                    }
                }

                // Now we're confident all messages are there, too; enact!
                self.handle.read(&((super::amdex::ID.into(), "utils".into()), "entry-count".into())).cast()?;
                let patients: Option<Vec<u8>> = self.handle.read(&((self.id().into(), "patients-2024".into()), "patients".into())).cast()?;
                self.handle
                    .write(
                        ((super::amy::ID.into(), "count-patients".into()), "num-patients".into()),
                        patients.map(|p| String::from_utf8_lossy(&p).lines().count()).unwrap_or(0).to_string().as_bytes(),
                    )
                    .cast()?;
                view.enacted.add(Selector::All, create_action(1, self.id(), agree.clone(), just)).cast()?;
                return Ok(Poll::Ready(()));
            },
        }
    }
}
