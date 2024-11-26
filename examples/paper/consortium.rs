//  CONSORTIUM.rs
//    by Lut99
//
//  Created:
//    27 May 2024, 17:42:39
//  Last edited:
//    26 Nov 2024, 11:54:58
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the administrator-agent from the paper.
//

use std::convert::Infallible;

use datalog::ast::{datalog, Reserializable, Spec};
use justact::agents::{Agent, AgentPoll, RationalAgent};
use justact::agreements::{Agreement, Agreements};
use justact::auxillary::Identifiable;
use justact::statements::Statements;
use justact::times::Times;
use justact_prototype::statements::{Message, Target};


/***** LIBRARY *****/
/// The consortium agent, dictating agreements.
#[derive(Debug)]
pub struct Consortium;
impl Identifiable for Consortium {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "consortium" }
}
impl Agent for Consortium {}
impl RationalAgent for Consortium {
    type Message = Message;
    type Target = Target;
    type Error = Infallible;

    fn poll(
        &mut self,
        mut agrs: impl Agreements<Message = Self::Message>,
        times: impl Times,
        _stmts: impl Statements<Message = Self::Message, Target = Self::Target>,
    ) -> Result<AgentPoll, Self::Error> {
        // The consortium emits 's1' at the start of the interaction
        if !agrs.agreed().contains("s1") {
            // Define the policy to emit
            let spec: Spec = datalog! {
                owns(administrator, Data) :- ctl_accesses(Accessor, Data).
                error :- ctl_accesses(Accessor, Data), owns(Owner, Data), not ctl_authorises(Owner, Accessor, Data).
            };
            let msg: Message = Message { id: "s1".into(), author: "consortium".into(), payload: spec.reserialize().to_string().into_bytes() };

            // Emit it
            agrs.agree(Agreement { msg, timestamp: times.current() }).unwrap();

            // The admin is done for this example
            return Ok(AgentPoll::Dead);
        }

        // That's it, this agent is done for the day
        Ok(AgentPoll::Alive)
    }
}
