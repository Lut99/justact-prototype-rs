//  AMDEX.rs
//    by Lut99
//
//  Created:
//    15 Jan 2025, 15:22:02
//  Last edited:
//    17 Jan 2025, 15:32:27
//  Auto updated?
//    Yes
//
//  Description:
//!   Describes the behaviour of the `amdex` agent as introduced in
//!   section 5.4.1 in the paper \[1\].
//

use std::error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Selector;
use justact::collections::map::{Map, MapAsync};
use justact::messages::ConstructableMessage;
use justact::times::Times;


/***** ERRORS *****/
#[derive(Debug)]
pub struct Error {
    /// The actual error produced.
    err: Box<dyn error::Error>,
}
impl Error {
    /// Constructor for the Error that we need because `From<E>` overlaps with Self >:(
    ///
    /// # Arguments
    /// - `err`: Some error to wrap.
    ///
    /// # Returns
    /// A new Error that behaves exactly as `err` but obfuscates its type.
    #[inline]
    pub fn new(err: impl 'static + error::Error) -> Self { Self { err: Box::new(err) } }
}
impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult { self.err.fmt(f) }
}
impl error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> { self.err.source() }
}





/***** LIBRARY *****/
pub struct Amdex;
impl Identifiable for Amdex {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "amdex" }
}
impl Agent<(String, u32), (String, u32), str, u64> for Amdex {
    type Error = Error;

    #[inline]
    fn poll<T, A, S, E, SM, SA>(&mut self, mut view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: Times<Timestamp = u64>,
        A: Map<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = str>,
        SA: ConstructableAction<Id = (String, u32), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        // The AMdEX agent can publish immediately, it doesn't yet need the agreement for just
        // stating.
        let id: (String, u32) = (self.id().into(), 1);
        match view.stated.contains_key(&id) {
            Ok(true) => Ok(Poll::Ready(())),
            Ok(false) => {
                // Push the message
                view.stated
                    .add(Selector::All, SM::new((String::new(), id.1), id.0.clone(), include_str!("../slick/amdex_1.slick").into()))
                    .map_err(Error::new)?;
                Ok(Poll::Ready(()))
            },
            Err(err) => Err(Error::new(err)),
        }
    }
}
