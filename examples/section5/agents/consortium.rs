//  CONSORTIUM.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:48:35
//  Last edited:
//    14 Jan 2025, 17:18:18
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a consortium [`Synchronizer`] as described in section 5.3
//!   and 5.4 of the JustAct paper \[1\].
//

use std::error;
use std::marker::PhantomData;
use std::ops::ControlFlow;

use justact::actions::Action;
use justact::actors::{Synchronizer, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::map::{MapAsync, MapSync};
use justact::collections::set::InfallibleSet as _;
use justact::messages::Message;
use justact::policies::Policy;
use justact::times::TimesSync;
use thiserror::Error;


/***** ERRORS *****/
/// The errors published by the Consortium.
#[derive(Debug, Error)]
pub enum Error {
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
pub struct Consortium<P> {
    /// The policy that is used by the consortium.
    _policy: PhantomData<P>,
}
impl<P> Identifiable for Consortium<P> {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "consortium" }
}
impl<P> Synchronizer<str, str, str, u128> for Consortium<P>
where
    P: Policy,
{
    type Error = Error;

    #[inline]
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<ControlFlow<()>, Self::Error>
    where
        T: TimesSync<Timestamp = u128>,
        A: MapSync<Agreement<SM, u128>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: Message<Id = str, AuthorId = Self::Id, Payload = str>,
        SA: Action<Id = str, ActorId = Self::Id, Message = SM, Timestamp = u128>,
    {
        // When no time is active yet, the consortium agent will initialize the system by bumping
        // it to `1` and making the initial agreement active.
        let current_times = view.times.current().map_err(|err| Error::TimesCurrent { err: Box::new(err) })?;
        if !current_times.contains(&1) {
            // Add the agreement
            todo!();

            // Update the timestamp
            view.times.add_current(1).map_err(|err| Error::TimesAddCurrent { timestamp: 1, err: Box::new(err) })?;
        }

        // Done, other agents can have a go
        Ok(ControlFlow::Continue(()))
    }
}
