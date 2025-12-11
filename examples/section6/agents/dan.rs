//  DAN.rs
//    by Lut99
//
//  Created:
//    21 Jan 2025, 09:25:37
//  Last edited:
//    31 Jan 2025, 16:15:58
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the "Disruptor" Dan agent from section 6.3.1 in the
//!   paper.
//

use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::Recipient;
use justact::collections::map::{Map, MapAsync};
use justact::messages::ConstructableMessage;
use justact::times::Times;
pub use justact_prototype::events::Error;
use justact_prototype::events::EventHandler;
use slick::Program;

use super::{Script, create_message};


/***** CONSTANTS *****/
/// This agent's ID.
#[allow(unused)]
pub const ID: &'static str = "dan";





/***** LIBRARY *****/
/// The `dan`-agent from section 6.3.1.
pub struct Dan {
    handler: EventHandler,
}
impl Dan {
    /// Constructor for the `dan` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what Dan will do.
    ///
    /// # Returns
    /// A new Dan agent.
    #[inline]
    #[allow(unused)]
    pub fn new(script: Script) -> Self {
        if !matches!(script, Script::Section6_3_1 | Script::Section6_3_3_ok | Script::Section6_3_3_crash) {
            panic!("Dan only plays a role in sections 6.3.1 and 6.3.3")
        }
        Self { handler: EventHandler::new() }
    }
}
impl Identifiable for Dan {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "dan" }
}
impl Agent<(String, u32), (String, char), Program, u64> for Dan {
    type Error = Error;

    #[track_caller]
    fn poll<T, A, S, E, SM, SA>(&mut self, view: View<T, A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        T: Times<Timestamp = u64>,
        A: Map<Agreement<SM, u64>>,
        S: MapAsync<Self::Id, SM>,
        E: MapAsync<Self::Id, SA>,
        SM: ConstructableMessage<Id = (String, u32), AuthorId = Self::Id, Payload = Program>,
        SA: ConstructableAction<Id = (String, char), ActorId = Self::Id, Message = SM, Timestamp = u64>,
    {
        self.handler.handle(view)
            // Dan waits for all the agreements and messages that precede him in the paper to be
            // sent first.
            .on_agreed_and_stated(
                (super::consortium::ID, 1),
                [
                    (super::surf::ID, 1),
                    (super::amy::ID, 1),
                    (super::st_antonius::ID, 1)
                ],
                |view, _, _| view.stated.add(Recipient::All, create_message(1, ID, slick::parse::program(include_str!("../slick/dan_1.slick")).unwrap().1))
            )?
            .finish()
    }
}
