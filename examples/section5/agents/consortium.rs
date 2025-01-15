//  CONSORTIUM.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:48:35
//  Last edited:
//    15 Jan 2025, 17:54:29
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a consortium [`Synchronizer`] as described in section 5.3
//!   and 5.4 of the JustAct paper \[1\].
//

use std::error;
use std::ops::ControlFlow;

use justact::actions::Action;
use justact::actors::{Synchronizer, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::map::{MapAsync, MapSync};
use justact::collections::set::InfallibleSet as _;
use justact::messages::Message;
use justact::times::TimesSync;
use log::debug;
use thiserror::Error;


/***** ERRORS *****/
/// The errors published by the Consortium.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to add to the current set of agreements.
    #[error("Failed to add agreements {author_id:?} {id:?}")]
    AgreementsAdd { id: String, author_id: String, err: Box<dyn error::Error> },
    /// Failed to add to the current set of times.
    #[error("Failed to add current time {timestamp:?}")]
    TimesAddCurrent { timestamp: u128, err: Box<dyn error::Error> },
    /// Failed to get the current set of times.
    #[error("Failed to get the set of current times")]
    TimesCurrent { err: Box<dyn error::Error> },
}





/***** LIBRARY *****/
/// Defines the consortium [`Synchronizer`], which has the power to define agreements and the
/// current time.
pub struct Consortium {
    /// The agreement published by the consortium.
    agreement: &'static str,
}
impl Consortium {
    /// Constructor for the consortium.
    ///
    /// # Arguments
    /// - `agreement`: The agreement (as a string Slick spec) that the consortium will publish.
    #[inline]
    pub const fn new(agreement: &'static str) -> Self { Self { agreement } }
}
impl Identifiable for Consortium {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "consortium" }
}
impl Synchronizer<(String, u32), (String, u32), str, u128> for Consortium {
    type Error = Error;

    #[inline]
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<ControlFlow<()>, Self::Error>
    where
        T: TimesSync<Timestamp = u128>,
        A: MapSync<Agreement<SM, u128>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: Message<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: Action<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u128>,
    {
        // When no time is active yet, the consortium agent will initialize the system by bumping
        // it to `1` and making the initial agreement active.
        let current_times = view.times.current().map_err(|err| Error::TimesCurrent { err: Box::new(err) })?;
        if !current_times.contains(&1) {
            // Add the agreement
            let agree = Agreement { message: SM::new((String::new(), 1), self.id().into(), self.agreement.into()), at: 1 };
            view.agreed.add(agree).map_err(|err| Error::AgreementsAdd { id: "1".into(), author_id: self.id().into(), err: Box::new(err) })?;
            debug!(target: "std::collections::HashMap<justact::agreement::Agreement<justact_prototype::wire::Message, u128>>", "Published new agreement {:?}", "1");

            // Update the timestamp
            view.times.add_current(1).map_err(|err| Error::TimesAddCurrent { timestamp: 1, err: Box::new(err) })?;
        }

        // Done, other agents can have a go
        Ok(ControlFlow::Continue(()))
    }
}
