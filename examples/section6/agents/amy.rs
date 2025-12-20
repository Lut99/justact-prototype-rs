//  AMY.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 15:11:36
//  Last edited:
//    31 Jan 2025, 16:23:39
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the `amy` agent from section 5.4 in the paper.
//

use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::auxillary::Identifiable;
use justact::collections::Recipient;
use justact::collections::set::{Set, SetAsync};
use justact::messages::{ConstructableMessage, MessageSet};
use justact_prototype::dataplane::{ScopedStoreHandle, StoreHandle};
// pub use justact_prototype::events::Error;
use justact_prototype::events::{Handler, ScriptBuilder};
use slick::text::Text;
use slick::{GroundAtom, Program};
use thiserror::Error;

use super::{Script, create_action, create_message};


/***** CONSTANTS *****/
/// This agent's ID.
pub const ID: &'static str = "amy";





/***** ERRORS *****/
#[derive(Debug, Error)]
pub enum Error {}





/***** LIBRARY *****/
/// The `amy`-agent from section 6.3.1.
pub struct Amy {
    script: Box<dyn Handler<Context = View>>,
    store:  ScopedStoreHandle,
}
impl Amy {
    /// Constructor for the `amy` agent.
    ///
    /// # Arguments
    /// - `script`: A [`Script`] describing what Amy will do.
    /// - `store`: A [`StoreHandle`] that this agent can use to interact with the world. It will
    ///   clone it internally, creating its own handle to the underlying store, meaning that the
    ///   dataplane handle can be dropped.
    ///
    /// # Returns
    /// A new Amy agent.
    #[inline]
    #[track_caller]
    #[allow(unused)]
    pub fn new(script: Script, store: &StoreHandle) -> Self {
        // Build the script first
        let script: H = match script {
            Script::Section6_3_1 => ScriptBuilder::new().run(|view| Ok(())).finish(),
            Script::Section6_3_3_ok => todo!(),
            Script::Section6_3_3_crash => todo!(),
            _ => panic!("Amy only plays a role in sections 6.3.1 and 6.3.3"),
        };

        // Then build ourselves
        Self { script, store: store.scope(ID) }
    }
}
impl<ERR> Identifiable for Amy<ERR> {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { ID }
}
impl<ERR> Agent<Program> for Amy<ERR> {
    type Error = ERR;

    #[track_caller]
    fn poll<A, S, E, SM, SA>(&mut self, view: View<A, S, E>) -> Result<Poll<()>, Self::Error>
    where
        A: Set<SM>,
        S: SetAsync<Self::Id, SM>,
        E: SetAsync<Self::Id, SA>,
        SM: ConstructableMessage<AuthorId = Self::Id, Payload = Program>,
        SA: ConstructableAction<ActorId = Self::Id, Message = SM>,
    {
        // In the third example, where Amy crashes, she just dies from the get-go :/
        if self.script == Script::Section6_3_3_crash {
            return Ok(Poll::Ready(()));
        }

        // Prepare some Slick facts to use
        let surf_utils = GroundAtom::Tuple(vec![
            GroundAtom::Tuple(vec![GroundAtom::Constant(Text::from_str(super::surf::ID)), GroundAtom::Constant(Text::from_str("utils"))]),
            GroundAtom::Constant(Text::from_str("executed")),
        ]);

        // // Encode Amy's event handler script
        // self.handler
        //     .handle_with_store(view, self.store.clone())
        //     // In the first scenario, Amy publishes her execution of `entry-count` on the St.
        //     // Antonius' dataset.
        //     // She only does that once she knows the package exists. As such, she waits until she
        //     // sees: `(surf utils) ready.` before she publishes `amy 1`.
        //     .on_truth(surf_utils, |view| view.stated.add(Recipient::All, create_message(1, ID, slick::parse::program(include_str!("../slick/amy_1.slick")).unwrap().1)))?
        //     // Then she waits until the St. Antonius has executed her task. Once so, she publishes
        //     // her intent to download the result (`amy 2`).
        //     .on_enacted((super::st_antonius::ID, 'b'), |view, _| view.stated.add(Recipient::All, create_message(2, ID, slick::parse::program(include_str!("../slick/amy_2.slick")).unwrap().1)))?
        //     // Finally, once she's gotten St. Antonius' authorisation to execute `amy 2`, she'll
        //     // collect the agreement and all statements (except Dan's) and enact it.
        //     .on_agreed_and_stated(
        //         (super::consortium::ID, 1),
        //         [
        //             (super::amy::ID, 1),
        //             (super::amy::ID, 2),
        //             (super::st_antonius::ID, 1),
        //             (super::st_antonius::ID, 2),
        //             (super::st_antonius::ID, 3),
        //             (super::surf::ID, 1),
        //         ],
        //         |view, agree, just| -> Result<(), Error> {
        //             view.enacted.add(Recipient::All, create_action('a', ID, agree, MessageSet::from_iter(just))).cast()?;
        //             self.store.read(((ID, "count-patients"), "num-patients"), (ID, 'a')).cast()?;
        //             Ok(())
        //         })?
        //     .finish()
        Ok(())
    }
}
