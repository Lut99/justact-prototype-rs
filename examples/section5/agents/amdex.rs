//  AMDEX.rs
//    by Lut99
//
//  Created:
//    15 Jan 2025, 15:22:02
//  Last edited:
//    15 Jan 2025, 17:54:55
//  Auto updated?
//    Yes
//
//  Description:
//!   Describes the behaviour of the `amdex` agent as introduced in
//!   section 5.4.1 in the paper \[1\].
//

use std::error;
use std::task::Poll;

use justact::actions::Action;
use justact::actors::{Agent, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Selector;
use justact::collections::map::{Map, MapAsync};
use justact::messages::Message;
use justact::times::Times;
use thiserror::Error;


/***** ERRORS *****/
/// The errors published by the Amdex agent.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to state message \"{} {}\"", id.0, id.1)]
    StatementsAdd {
        id:  (String, u32),
        #[source]
        err: Box<dyn error::Error>,
    },
    #[error("Failed to check for statement \"{} {}\" existance", id.0, id.1)]
    StatementsContainsKey {
        id:  (String, u32),
        #[source]
        err: Box<dyn error::Error>,
    },
}





/***** LIBRARY *****/
pub struct Amdex;
impl Identifiable for Amdex {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "amdex" }
}
impl Agent<(String, u32), (String, u32), str, u128> for Amdex {
    type Error = Error;

    #[inline]
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: Times<Timestamp = u128>,
        A: Map<Agreement<SM, u128>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: Message<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: Action<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u128>,
    {
        // The AMdEX agent can publish immediately, it doesn't yet need the agreement for just
        // stating.
        let id: (String, u32) = (self.id().into(), 1);
        match view.stated.contains_key(&id) {
            Ok(true) => Ok(Poll::Ready(())),
            Ok(false) => {
                // Push the message
                view.stated
                    .add(Selector::All, SM::new((String::new(), id.1), id.0.clone(), r#""#.into()))
                    .map_err(|err| Error::StatementsAdd { id, err: Box::new(err) })?;
                Ok(Poll::Ready(()))
            },
            Err(err) => Err(Error::StatementsContainsKey { id, err: Box::new(err) }),
        }
    }
}
