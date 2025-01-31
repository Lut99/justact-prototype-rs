//  CONSORTIUM.rs
//    by Lut99
//
//  Created:
//    14 Jan 2025, 16:48:35
//  Last edited:
//    31 Jan 2025, 17:36:55
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
use justact::messages::ConstructableMessage;
use justact::times::TimesSync;
pub use justact_prototype::events::Error;
use justact_prototype::events::{EventHandler, ResultToError as _};

use super::{Script, create_message};


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "consortium";





/***** LIBRARY *****/
/// The `consortium`-agent from section 6.3.1.
pub struct Consortium {
    script:  Script,
    handler: EventHandler,
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
    pub fn new(script: Script) -> Self { Self { script, handler: EventHandler::new() } }
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
    fn poll<T, A, S, E, SM, SA>(&mut self, view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: TimesSync<Timestamp = u64>,
        A: MapSync<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, char), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        let mut handler = self.handler.handle(view)
            // At the start immediately, the consortium agent will initialize the system by bumping the
            // active time to `1` and making the initial agreement active.
            .on_start(|view| -> Result<(), Error> {
                let message: SM = create_message(1, ID, include_str!("../slick/consortium_1.slick"));
                view.stated.add(Recipient::All, message.clone()).cast()?;
                view.agreed.add(Agreement { message, at: 1 }).cast()?;
                view.times.add_current(1).cast()?;
                
                // Done
                Ok(())
            })?;

        // In section 6.3.5, we do some additional stuff
        if self.script == Script::Section6_3_5 {
            handler = handler
                // After the St. Antonius has done some work, we will amend the agreement.
                .on_enacted((super::st_antonius::ID, 'a'), |view, _| -> Result<(), Error> {
                    // Push the amendment
                    let message: SM = create_message(2, ID, include_str!("../slick/consortium_2.slick"));
                    view.stated.add(Recipient::All, message.clone()).cast()?;
                    view.agreed.add(Agreement { message, at: 2 }).cast()?;
                    view.times.add_current(2).cast()?;
                    Ok(())
                })?
                // Again, after the St. Antonius did things, something illegal happened! We pull
                // the wires out of the system.
                .on_stated((super::st_antonius::ID, 7), |view, _| -> Result<(), Error> {
                    view.times.add_current(3).cast()?;
                    Ok(())
                })?
                // Finally, once the previous step has happened, some times has passed and we re-
                // instate the previous agreement.
                .on_tick_to(3, |view| -> Result<(), Error> {
                    view.agreed.add(Agreement { message: create_message(2, ID, include_str!("../slick/consortium_2.slick")), at: 3 }).cast()?;
                    Ok(())
                })?;
        }

        // Otherwise, done
        handler.finish()
    }
}
