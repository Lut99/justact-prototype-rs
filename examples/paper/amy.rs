//  AMY.rs
//    by Lut99
//
//  Created:
//    27 May 2024, 17:53:21
//  Last edited:
//    26 Nov 2024, 11:54:47
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the amy-agent from the paper.
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
/// The amy agent, wanting to do work.
#[derive(Debug)]
pub struct Amy;
impl Identifiable for Amy {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "amy" }
}
impl Agent for Amy {}
impl RationalAgent for Amy {
    type Message = Message;
    type Target = Target;
    type Error = Infallible;

    fn poll(
        &mut self,
        agrs: impl Agreements<Message = Self::Message>,
        times: impl Times,
        mut stmts: impl Statements<Message = Self::Message, Target = Self::Target>,
    ) -> Result<AgentPoll, Self::Error> {
        // The amy emits 's3' (an enacted action) after she received authorisation from the amy
        if stmts.stated().contains("s2") {
            // Amy first emits her intended enactment
            {
                // The policy to emit
                let spec: Spec = datalog! {
                    ctl_accesses(amy, x_rays).
                };
                let msg: Message = Message { id: "s3".into(), author: "amy".into(), payload: spec.reserialize().to_string().into_bytes() };

                // Emit it
                stmts.state(Target::All, msg);
            }

            // Then, she creates an Action
            {
                // The action to emit
                let act: Action<Message> = Action {
                    basis:     (*agrs.agreed().get("s1").unwrap()).clone(),
                    just:      LocalSet::from([(*stmts.stated().get("s2").unwrap()).clone()]),
                    enacts:    (*stmts.stated().get("s3").unwrap()).clone(),
                    timestamp: times.current(),
                };

                // Emit it
                stmts.enact(Target::All, act);
            }

            // That's Amy's role
            return Ok(AgentPoll::Dead);
        }

        // That's it, this agent is done for the day
        Ok(AgentPoll::Alive)
    }
}
