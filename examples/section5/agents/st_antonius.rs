//  ST ANTONIUS.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 17:45:04
//  Last edited:
//    17 Jan 2025, 17:51:32
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the St. Antonius agent from section 5.4 in the JustAct
//!   paper \[1\].
//

use std::error;
use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Selector;
use justact::collections::map::{Map, MapAsync};
use justact::messages::ConstructableMessage;
use justact::times::Times;
use thiserror::Error;


/***** ERRORS *****/
/// Defines errors originating in the [`Amy`] agent.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to check if statement \"{} {}\" is stated", id.0, id.1)]
    StatementContains {
        id:  (String, u32),
        #[source]
        err: Box<dyn error::Error>,
    },
    #[error("Failed to state new statement \"{} {}\"", id.0, id.1)]
    StatementState {
        id:  (String, u32),
        #[source]
        err: Box<dyn error::Error>,
    },
}
impl Error {
    /// Constructor for [`Error::StatementContains`] that makes it convenient to map.
    ///
    /// # Arguments
    /// - `err`: The [`error::Error`] to wrap.
    ///
    /// # Returns
    /// A new [`Error::StatementContains`].
    #[inline]
    pub fn contains(id: (String, u32), err: impl 'static + error::Error) -> Self { Self::StatementContains { id: id.into(), err: Box::new(err) } }

    /// Constructor for [`Error::StatementState`] that makes it convenient to map.
    ///
    /// # Arguments
    /// - `err`: The [`error::Error`] to wrap.
    ///
    /// # Returns
    /// A new [`Error::StatementState`].
    #[inline]
    pub fn state(id: (String, u32), err: impl 'static + error::Error) -> Self { Self::StatementState { id, err: Box::new(err) } }
}





/***** LIBRARY *****/
/// The `st-antonius`-agent from section 5.4.1.
pub struct StAntonius;
impl Identifiable for StAntonius {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "st-antonius" }
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
        // The St. Antonius publishes their authorization only after Amy has published
        let amy_id: (String, u32) = ("amy".into(), 1);
        if view.stated.contains_key(&amy_id).map_err(|err| Error::contains(amy_id, err))? {
            // Publish ours
            let id: (String, u32) = (self.id().into(), 1);
            view.stated
                .add(Selector::All, SM::new((String::new(), id.1), id.0.clone(), include_str!("../slick/st-antonius_1.slick").into()))
                .map_err(|err| Error::state(id, err))?;
            return Ok(Poll::Ready(()));
        }

        // Else, keep waiting
        Ok(Poll::Pending)
    }
}
