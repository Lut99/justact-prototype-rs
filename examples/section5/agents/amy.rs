//  AMY.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 15:11:36
//  Last edited:
//    17 Jan 2025, 17:42:03
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the `amy` agent from section 5.4 in the paper.
//

use std::error;
use std::task::Poll;

use justact::actions::ConstructableAction;
use justact::actors::{Agent, View};
use justact::agreements::Agreement;
use justact::auxillary::Identifiable;
use justact::collections::map::{Map, MapAsync};
use justact::collections::set::InfallibleSet;
use justact::collections::{Selector, Singleton};
use justact::messages::ConstructableMessage;
use justact::policies::{Extractor, Policy as _};
use justact::times::Times;
use justact_prototype::policy::slick::{Denotation, Extractor as SlickExtractor};
use slick::GroundAtom;
use slick::text::Text;
use thiserror::Error;


/***** ERRORS *****/
/// Defines errors originating in the [`Amy`] agent.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to extract Slick policy from message \"{} {}\"", id.0, id.1)]
    ExtractSlick {
        id:  (String, u32),
        #[source]
        err: Box<dyn error::Error>,
    },
    #[error("Failed to iterate over statements")]
    StatementsIter {
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
    /// Constructor for [`Error::ExtractSlick`] that makes it convenient to map.
    ///
    /// # Arguments
    /// - `err`: The [`error::Error`] to wrap.
    ///
    /// # Returns
    /// A new [`Error::ExtractSlick`].
    #[inline]
    pub fn extract(id: (String, u32), err: impl 'static + error::Error) -> Self { Self::ExtractSlick { id: id.into(), err: Box::new(err) } }

    /// Constructor for [`Error::StatementState`] that makes it convenient to map.
    ///
    /// # Arguments
    /// - `err`: The [`error::Error`] to wrap.
    ///
    /// # Returns
    /// A new [`Error::StatementState`].
    #[inline]
    pub fn state(id: (String, u32), err: impl 'static + error::Error) -> Self { Self::StatementState { id, err: Box::new(err) } }

    /// Constructor for [`Error::StatementsIter`] that makes it convenient to map.
    ///
    /// # Arguments
    /// - `err`: The [`error::Error`] to wrap.
    ///
    /// # Returns
    /// A new [`Error::StatementsIter`].
    #[inline]
    pub fn stmts_iter(err: impl 'static + error::Error) -> Self { Self::StatementsIter { err: Box::new(err) } }
}





/***** LIBRARY *****/
/// The `amy`-agent from section 5.4.1.
pub struct Amy;
impl Identifiable for Amy {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { "amy" }
}
impl Agent<(String, u32), (String, u32), str, u64> for Amy {
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
        // Amy waits until she sees her package of interest pop into existance
        let pkg = GroundAtom::Tuple(vec![
            GroundAtom::Tuple(vec![GroundAtom::Constant(Text::from_str("amdex")), GroundAtom::Constant(Text::from_str("utils"))]),
            GroundAtom::Constant(Text::from_str("ready")),
        ]);
        let mut state: bool = false;
        for msg in view.stated.iter().map_err(Error::stmts_iter)? {
            let set = Singleton(msg);
            let denot: Denotation = SlickExtractor.extract(&set).map_err(|err| Error::extract(msg.id().clone(), err))?.truths();
            if denot.is_valid() && <Denotation as InfallibleSet<GroundAtom>>::contains(&denot, &pkg) {
                // The message exists (and is valid)! Publish her snippet.
                state = true;
                break;
            }
        }

        // Publish if we found the target message; else keep waiting
        if state {
            // Push the message
            let id: (String, u32) = (self.id().into(), 1);
            view.stated
                .add(Selector::All, SM::new((String::new(), id.1), id.0.clone(), include_str!("../slick/amy_1.slick").into()))
                .map_err(|err| Error::state(id, err))?;
            Ok(Poll::Ready(()))
        } else {
            // Amy's not done otherwise
            Ok(Poll::Pending)
        }
    }
}
