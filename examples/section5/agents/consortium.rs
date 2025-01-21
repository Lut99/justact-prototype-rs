//  CONSORTIUM.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:48:35
//  Last edited:
//    21 Jan 2025, 09:56:21
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a consortium [`Synchronizer`] as described in section 5.3
//!   and 5.4 of the JustAct paper \[1\].
//

use std::ops::ControlFlow;

use justact::actions::ConstructableAction;
use justact::actors::{Synchronizer, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::map::{MapAsync, MapSync};
use justact::collections::set::InfallibleSet as _;
use justact::messages::ConstructableMessage;
use justact::times::TimesSync;

use super::create_message;
pub use crate::error::Error;
use crate::error::ResultToError as _;


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
impl Synchronizer<(String, u32), (String, u32), str, u64> for Consortium {
    type Error = Error;

    #[inline]
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<ControlFlow<()>, Self::Error>
    where
        T: TimesSync<Timestamp = u64>,
        A: MapSync<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        // When no time is active yet, the consortium agent will initialize the system by bumping
        // it to `1` and making the initial agreement active.
        let current_times = view.times.current().cast()?;
        if !current_times.contains(&1) {
            // Add the agreement
            let agree = Agreement { message: create_message(1, self.id(), self.agreement), at: 1 };
            view.agreed.add(agree).cast()?;

            // Update the timestamp
            view.times.add_current(1).cast()?;
        }

        // Done, other agents can have a go
        Ok(ControlFlow::Continue(()))
    }
}
