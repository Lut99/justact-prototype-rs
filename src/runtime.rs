//  RUNTIME.rs
//    by Lut99
//
//  Created:
//    13 Jan 2025, 15:05:42
//  Last edited:
//    13 Jan 2025, 17:22:58
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the main runtime regarding the JustAct policy engine.
//

use std::collections::HashSet;
use std::sync::Arc;

use crate::sets::MapAsync;
use crate::wire::{Action, Message};

mod justact {
    pub use ::justact::actors::{Agent, Synchronizer};
    pub use ::justact::agreements::Agreement;
    pub use ::justact::runtime::Runtime;
}


/***** LIBRARY *****/
/// Defines the prototype runtime that will do things in-memory.
pub struct Runtime {}
impl justact::Runtime for Runtime {
    type AgentId = str;

    type Message = Arc<Message>;
    type Action = Action;

    type Times = HashSet<u128>;
    type Agreements = HashMap<String, justact::Agreement<Arc<Message>, u128>>;
    type Statements = MapAsync<Arc<Message>>;
    type Enactments = MapAsync<Action>;

    type Error = std::convert::Infallible;


    #[inline]
    fn run<A>(&mut self, agents: impl IntoIterator<Item = A>, synchronizer: impl justact::Synchronizer) -> Result<(), Self::Error>
    where
        A: justact::Agent,
    {
        todo!()
    }
}
