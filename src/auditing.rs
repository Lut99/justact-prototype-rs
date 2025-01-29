//  VALIDITY.rs
//    by Lut99
//
//  Created:
//    29 Jan 2025, 21:14:36
//  Last edited:
//    29 Jan 2025, 23:22:28
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements logic to compute the validity of a given action.
//

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ::justact::collections::map::InfallibleMap as _;
use ::justact::policies::{Denotation as _, Policy as _};
use slick::GroundAtom;
use slick::text::Text;

use crate::policy::slick::{AffectorAtom, Denotation, Effect, Extractor, PatternAtom, SyntaxError};
use crate::wire::{Action, Agreement, Message};

mod justact {
    pub use ::justact::collections::Recipient;
}


/***** AUXILLARY *****/
/// Defines how we describe the validity of an action.
#[derive(Clone, Debug)]
pub struct Permission {
    /// Definition 3.3, part 1: stated justification.
    ///
    /// I.e., all messages in the justification are stated at the time of enacting.
    pub stated:  bool,
    /// Definition 3.3, part 2: based justification.
    ///
    /// I.e., the message contained in the basis is also in the justification.
    pub based:   bool,
    /// Definition 3.3, part 3: valid justification.
    ///
    /// I.e., the justification's denotation does not derive `error`.
    pub valid:   bool,
    /// Definition 3.4, part 4: current action.
    ///
    /// I.e., the basis is at a current time at the time of enacting.
    pub current: bool,

    /// Describes the truths denoted by this action.
    ///
    /// For convenience, sorted by: errors first (alphabetically), then other truths
    /// (alphabetically).
    pub truths:  Vec<GroundAtom>,
    /// Describes the effects denoted by this action.
    ///
    /// For convenience, sorted by alphabet.
    pub effects: Vec<Effect>,
}
impl Default for Permission {
    #[inline]
    fn default() -> Self { Self { stated: true, based: true, valid: true, current: true, truths: Vec::new(), effects: Vec::new() } }
}
impl Permission {
    /// Checks whether the action represented by this permission is permitted.
    ///
    /// # Returns
    /// True if it's a correctly justified action, or false otherwise.
    #[inline]
    pub const fn is_permitted(&self) -> bool { self.stated && self.based && self.valid && self.current }
}



/// Defines an event in a trace of events that, together, make up an auditable log of the system.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Event<'a> {
    /// Defines events eminating from the JustAct framework.
    Control(EventControl<'a>),
    /// Defines events eminating from the data plane.
    #[cfg(feature = "dataplane")]
    Data(EventData<'a>),
}

/// Defines what may be traced by the JustAct-part of the framework (governance).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind"))]
pub enum EventControl<'a> {
    /// Traces the addition of a new agreement.
    AddAgreement { agree: Agreement },
    /// Traces the advancement of the current time.
    AdvanceTime { timestamp: u64 },
    /// Traces the enacting of an action.
    EnactAction { who: Cow<'a, str>, to: justact::Recipient<Cow<'a, str>>, action: Action },
    /// States a new message.
    StateMessage { who: Cow<'a, str>, to: justact::Recipient<Cow<'a, str>>, msg: Arc<Message> },
}

/// Defines what may be traced by the dataplane-part of the framework (transactions).
#[derive(Clone, Debug)]
#[cfg(feature = "dataplane")]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind"))]
pub enum EventData<'a> {
    /// Traces that somebody read from a variable.
    Read { who: Cow<'a, str>, id: Cow<'a, ((String, String), String)>, context: (Cow<'a, str>, char), contents: Option<Cow<'a, [u8]>> },
    /// Traces that somebody wrote to a variable.
    Write { who: Cow<'a, str>, id: Cow<'a, ((String, String), String)>, context: (Cow<'a, str>, char), new: bool, contents: Cow<'a, [u8]> },
}





/***** LIBRARY *****/
/// Defines a so-called "audit" that is used to examine a [`Trace`] and properly asses action
/// validity in the context of the system at the time of enacting.
#[derive(Debug)]
pub struct Audit {
    /// The current number of events seen.
    i: usize,
    /// The list of stated messages up to this point.
    stated: HashSet<(String, u32)>,
    /// The current time at this point.
    current: Option<u64>,
    /// A list of event indices mapping [`EventControl::EnactAction`]s to [`Permission`]s.
    validity: HashMap<usize, Result<Permission, SyntaxError>>,
}

// Constructors
impl Default for Audit {
    #[inline]
    fn default() -> Self { Self::new() }
}
impl Audit {
    /// Creates a new Audit that is initialized to not having seen any trace yet.
    ///
    /// # Returns
    /// A new Audit ready for (wait for it) auditing.
    #[inline]
    pub fn new() -> Self { Self { i: 0, stated: HashSet::with_capacity(64), current: None, validity: HashMap::with_capacity(16) } }
}

// Auditing
impl Audit {
    /// Audits a particular [`Event`].
    ///
    /// # Arguments
    /// - `event`: An [`Event`] to examine. Will update the "current state" of the system the audit
    ///   keeps internally if it's an [`EventControl::AdvanceTime`] or an
    ///   [`EventControl::StateMessage`]. If it's an [`EventControl::EnactAction`], will store its
    ///   validity.
    pub fn audit(&mut self, event: &Event) {
        match event {
            // We're only interested in control plane events
            Event::Control(event) => match event {
                // We keep track of the current time & stated messages
                EventControl::AdvanceTime { timestamp } => {
                    self.current = Some(*timestamp);
                    self.i += 1;
                },
                EventControl::StateMessage { who: _, to: _, msg } => {
                    self.stated.insert(msg.id.clone());
                    self.i += 1;
                },

                // Enacting of actions triggers the "real" audit
                EventControl::EnactAction { who: _, to: _, action } => {
                    let mut validity: Permission = Default::default();

                    // Before we begin, compute the action's denotation
                    let denot: Denotation = match Extractor.extract_with_actor(&action.id.0, &action.justification) {
                        Ok(mut pol) => {
                            pol.update_effect_pattern(
                                PatternAtom::Tuple(vec![
                                    PatternAtom::Variable(Text::from_str("Worker")),
                                    PatternAtom::ConstantSet(vec![Text::from_str("reads"), Text::from_str("writes")]),
                                    PatternAtom::Variable(Text::from_str("Variable")),
                                ]),
                                AffectorAtom::Variable(Text::from_str("Worker")),
                            );
                            pol.truths()
                        },
                        Err(err) => {
                            // We failed to extract. Log the error.
                            self.validity.insert(self.i, Err(err));
                            self.i += 1;
                            return;
                        },
                    };
                    let mut truths: Vec<(bool, GroundAtom)> = denot
                        .iter_truths()
                        .cloned()
                        .map(|t| {
                            // First, find out which atoms are errors; then sort on that boolean
                            // first before we sort on the alphabet
                            (
                                match &t {
                                    GroundAtom::Constant(t) if format!("{t:?}") == "error" => true,
                                    GroundAtom::Tuple(ts) if !ts.is_empty() && format!("{:?}", ts[0]) == "error" => true,
                                    _ => false,
                                },
                                t,
                            )
                        })
                        .collect();
                    truths.sort_by(|lhs, rhs| match (lhs.0, rhs.0) {
                        (true, false) => Ordering::Less,
                        (false, true) => Ordering::Greater,
                        _ => format!("{:?}", lhs.1).cmp(&format!("{:?}", rhs.1)),
                    });
                    validity.truths = truths.into_iter().map(|(_, t)| t).collect();
                    validity.effects = denot.iter_effects().cloned().collect();
                    validity.effects.sort_by_key(|e| format!("{e:?}"));



                    // First property: check whether everything in the justification is stated
                    for msg in action.justification.iter() {
                        validity.stated &= self.stated.contains(&msg.id);
                    }

                    // Second property: is the basis in the justification?
                    validity.based = action.justification.contains_key(&action.basis.message.id);

                    // Third property: are the truths valid?
                    // NOTE: Because we have sorted truths already, the search should be crazy fast
                    validity.valid = denot.is_valid();

                    // Fourth property: is the basis current?
                    validity.current = Some(action.basis.at) == self.current;



                    // OK, cache the validity check & denotation
                    self.validity.insert(self.i, Ok(validity));
                    self.i += 1;
                },

                // Adding of agreements has no effect on us.
                EventControl::AddAgreement { agree: _ } => self.i += 1,
            },

            // Data events have no bearing on us
            #[cfg(feature = "dataplane")]
            Event::Data(_) => self.i += 1,
        }
    }
}

// Action retrieval
impl Audit {
    /// Attempts to find the action with the given index.
    ///
    /// # Returns
    /// Three things can happen:
    /// - [`Some(Ok(Permission { ... }))`](Permission) is returned, indicating that the given index
    ///   points to an audited action which who's justification was parsed successfully;
    /// - [`Some(Err(_))`](Err) is returned, indicating that the given index points to an audited
    ///   action but its justification did not result in a parsable policy; or
    /// - [`None`] is returned, indicating that no action was audited at the given index.
    pub fn permission_of(&self, index: usize) -> Option<&Result<Permission, SyntaxError>> { self.validity.get(&index) }
}
