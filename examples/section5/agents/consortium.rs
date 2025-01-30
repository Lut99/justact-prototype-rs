//  CONSORTIUM.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:48:35
//  Last edited:
//    30 Jan 2025, 20:48:10
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a consortium [`Synchronizer`] as described in section 5.3
//!   and 5.4 of the JustAct paper \[1\].
//

use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Synchronizer, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Recipient;
use justact::collections::map::{MapAsync, MapSync};
use justact::collections::set::InfallibleSet as _;
use justact::messages::ConstructableMessage;
use justact::times::TimesSync;

use super::{Script, create_message};
pub use crate::error::Error;
use crate::error::ResultToError as _;


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "consortium";





/***** HELPERS *****/
/// Defines the consortium's state for section 5.4.5.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum State5_4_5 {
    /// Publish the initial agreement.
    InitialAgreement,
    /// We changed our mind. Move to the secondary agreement.
    AmendedAgreement,
    /// Oh no, something happened! Time to pull the wires.
    PullOutWires,
    /// Order has been restored; go back to the amended agreement.
    BackForSeconds,
}





/***** LIBRARY *****/
/// The `consortium`-agent from section 5.4.1.
pub struct Consortium {
    script: Script,
    state:  State5_4_5,
}
impl Consortium {
    /// Constructor for the consortium.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what the consortium will do.
    ///
    /// # Returns
    /// A new Consortium agent.
    #[inline]
    pub fn new(script: Script) -> Self { Self { script, state: State5_4_5::InitialAgreement } }
}
impl Identifiable for Consortium {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl Synchronizer<(String, u32), (String, char), str, u64> for Consortium {
    type Error = Error;

    #[inline]
    #[track_caller]
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: TimesSync<Timestamp = u64>,
        A: MapSync<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, char), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        match self.script {
            Script::Section5_4_1 | Script::Section5_4_2 | Script::Section5_4_3 | Script::Section5_4_4 => {
                // When no time is active yet, the consortium agent will initialize the system by bumping
                // it to `1` and making the initial agreement active.
                let current_times = view.times.current().cast()?;
                if !current_times.contains(&1) {
                    // State the message
                    let message: SM = create_message(1, self.id(), include_str!("../slick/consortium_1.slick"));
                    view.stated.add(Recipient::All, message.clone()).cast()?;

                    // Add the agreement
                    view.agreed.add(Agreement { message, at: 1 }).cast()?;

                    // Update the timestamp
                    view.times.add_current(1).cast()?;
                }

                // We're done then
                Ok(Poll::Ready(()))
            },
            Script::Section5_4_5 => {
                match self.state {
                    State5_4_5::InitialAgreement => {
                        // When no time is active yet, the consortium agent will initialize the system by bumping
                        // it to `1` and making the initial agreement active.
                        let current_times = view.times.current().cast()?;
                        if !current_times.contains(&1) {
                            // State the message
                            let message: SM = create_message(1, self.id(), include_str!("../slick/consortium_1.slick"));
                            view.stated.add(Recipient::All, message.clone()).cast()?;

                            // Add the agreement
                            view.agreed.add(Agreement { message, at: 1 }).cast()?;

                            // Update the timestamp
                            view.times.add_current(1).cast()?;
                        }

                        // This time, move to the second state
                        self.state = State5_4_5::AmendedAgreement;
                        Ok(Poll::Pending)
                    },

                    State5_4_5::AmendedAgreement => {
                        // Once the St. Antonius has done their thing, we decide to amend the agreement
                        if !view.stated.contains_key(&(super::st_antonius::ID.into(), 1)).cast()? {
                            return Ok(Poll::Pending);
                        }

                        // Push the amendment
                        // State the message
                        let message: SM = create_message(2, self.id(), include_str!("../slick/consortium_2.slick"));
                        view.stated.add(Recipient::All, message.clone()).cast()?;
                        view.agreed.add(Agreement { message, at: 2 }).cast()?;

                        // Update the timestamp
                        view.times.add_current(2).cast()?;

                        // Then we move to the third state!
                        self.state = State5_4_5::PullOutWires;
                        Ok(Poll::Pending)
                    },

                    State5_4_5::PullOutWires => {
                        // To emulate time passing, wait for the St. Antonius to publish their message
                        if !view.stated.contains_key(&(super::st_antonius::ID.into(), 7)).cast()? {
                            return Ok(Poll::Pending);
                        }

                        // Move to time 3, but make nothing active!!
                        view.times.add_current(3).cast()?;

                        // Move to the final state
                        self.state = State5_4_5::BackForSeconds;
                        Ok(Poll::Pending)
                    },

                    State5_4_5::BackForSeconds => {
                        // We'll have to re-publish the same agreement at time 3, then everything is fine!
                        let agree = Agreement { message: create_message(2, self.id(), include_str!("../slick/consortium_2.slick")), at: 3 };
                        view.agreed.add(agree).cast()?;

                        // Done with this example
                        Ok(Poll::Ready(()))
                    },
                }
            },
        }
    }
}
