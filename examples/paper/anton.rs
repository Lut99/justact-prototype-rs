//  ANTON.rs
//    by Lut99
//
//  Created:
//    27 May 2024, 18:01:02
//  Last edited:
//    26 Nov 2024, 11:54:53
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the anton-agent from the paper.
//

use std::convert::Infallible;

use datalog::ast::{datalog, Reserializable, Spec};
use justact::agents::{Agent, AgentPoll, RationalAgent};
use justact::agreements::Agreements;
use justact::auxillary::Identifiable;
use justact::set::LocalSet;
use justact::statements::{Action, Statements};
use justact::times::Times;
use justact_prototype::statements::{Message, Target};


/***** LIBRARY *****/
/// The anton agent, being malicious.
#[derive(Debug)]
pub struct Anton;
impl Identifiable for Anton {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "anton" }
}
impl Agent for Anton {}
impl RationalAgent for Anton {
    type Message = Message;
    type Target = Target;
    type Error = Infallible;

    fn poll(
        &mut self,
        agrs: impl Agreements<Message = Self::Message>,
        times: impl Times,
        mut stmts: impl Statements<Message = Self::Message, Target = Self::Target>,
    ) -> Result<AgentPoll, Self::Error> {
        // Anton emits some malicious messages at the end
        if stmts.stated().contains("s3") && !stmts.stated().contains("s5") {
            // To illustrate, we also emit an action at the end
            {
                // Define the policy to emit
                let spec: Spec = datalog! {
                    ctl_authorises(administrator, anton, x_rays).
                };
                let msg: Message = Message { id: "s4".into(), author: "anton".into(), payload: spec.reserialize().to_string().into_bytes() };

                // Emit it
                stmts.state(Target::All, msg);
            }
            {
                // Define the policy to emit
                let spec: Spec = datalog! {
                    ctl_accesses(anton, x_rays).
                };
                let msg: Message = Message { id: "s5".into(), author: "anton".into(), payload: spec.reserialize().to_string().into_bytes() };

                // Emit it
                stmts.state(Target::All, msg);
            }
            {
                // The action to emit
                let act: Action<Message> = Action {
                    basis:     (*agrs.agreed().get("s1").unwrap()).clone(),
                    just:      LocalSet::from([(*stmts.stated().get("s4").unwrap()).clone()]),
                    enacts:    (*stmts.stated().get("s5").unwrap()).clone(),
                    timestamp: times.current(),
                };

                // Emit it
                stmts.enact(Target::All, act);
            }
        } else if stmts.stated().contains("s5") {
            // Define the policy to emit
            let spec: Spec = datalog! {
                owns(anton, x_rays).
            };
            let msg: Message = Message { id: "s6".into(), author: "anton".into(), payload: spec.reserialize().to_string().into_bytes() };

            // Emit it
            stmts.state(Target::All, msg);

            // That's Anton's work forever
            return Ok(AgentPoll::Dead);
        }

        // Wait until it's Anton's moment to shine
        Ok(AgentPoll::Alive)
    }
}
