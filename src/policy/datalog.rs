//  DATALOG.rs
//    by Lut99
//
//  Created:
//    26 Nov 2024, 11:54:14
//  Last edited:
//    17 Dec 2024, 17:03:14
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements JustAct traits for the [`datalog`]-crate.
//!   
//!   # Semantics
//!   - Effects are `effect(AFFECTOR, EFFECT)`.
//

use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;

use datalog::ast::{Atom, AtomArgs, Comma, Dot, Ident, Punctuated, Rule, Span, Spec};
use datalog::interpreter::interpretation::Interpretation;
use datalog::parser::parse;
use error_trace::trace;
use thiserror::Error;
mod justact {
    pub use ::justact::auxillary::{Affectored, Identifiable};
    pub use ::justact::messages::Message;
    pub use ::justact::policies::{Denotation, Effect, Extractor, Policy, Truth};
    pub use ::justact::sets::Set;
}


/***** ERRORS *****/
/// Defines errors that may occur when [validating](Policy::assert_validity()) policy.
#[derive(Debug, Error)]
pub enum SemanticError<'f, 's> {
    #[error("\"error\" holds in the interpretation\n\n{int}")]
    ErrorHolds { int: Interpretation<'f, 's> },
}

/// Defines errors that may occur when [extracting](Extractor::extract()) policy.
#[derive(Debug, Error)]
pub enum SyntaxError<'m> {
    #[error("{}", trace!(("Failed to parse the input as valid Datalog"), err))]
    Datalog { err: datalog::parser::Error<&'m str, &'m str> },
    #[error("{}", trace!(("Failed to iterate over messages in {what}"), &**err))]
    Iter { what: &'static str, err: Box<dyn 'm + Error> },
}





/***** LIBRARY *****/
/// Wraps a Datalog fact of a VERY particular shape as an [`Effect`](justact::Effect).
#[derive(Clone, Debug)]
pub struct Effect<'f, 's>(pub Truth<'f, 's>);
impl<'f, 's> justact::Affectored for Effect<'f, 's> {
    type AffectorId = Ident<&'f str, &'s str>;

    #[inline]
    fn affector_id(&self) -> &Self::AffectorId {
        // Attempt to parse the inner atom as `effect(AFFECTOR, EFFECT)`
        if self.0.fact.ident.value.value() != "effect" {
            panic!("Invalid effect atom: got {:?}, expected \"effect\"", self.0.fact.ident.value.value());
        } else if self.0.fact.args.is_none() {
            panic!("Invalid effect atom: expected 2 arguments, got none");
        }
        let args: &AtomArgs<&'f str, &'s str> = self.0.fact.args.as_ref().unwrap();
        if args.args.len() != 2 {
            panic!("Invalid effect atom: expected 2 arguments, got {}", args.args.len());
        }
        args.args[0].ident()
    }
}
impl<'f, 's> justact::Effect for Effect<'f, 's> {}
impl<'f, 's> justact::Identifiable for Effect<'f, 's> {
    type Id = Atom<&'f str, &'s str>;

    #[inline]
    fn id(&self) -> &Self::Id { self.0.id() }
}
impl<'f, 's> justact::Truth for Effect<'f, 's> {
    #[inline]
    fn value(&self) -> Option<bool> { self.0.value }
}

/// Wraps a Datalog (fact, truth) pair as a [`Truth`](justact::Truth).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Truth<'f, 's> {
    /// Defines the fact who's truth we are describing.
    pub fact:  Atom<&'f str, &'s str>,
    /// The truth value of the fact we're describing.
    pub value: Option<bool>,
}
impl<'f, 's> justact::Identifiable for Truth<'f, 's> {
    type Id = Atom<&'f str, &'s str>;

    #[inline]
    fn id(&self) -> &Self::Id { &self.fact }
}
impl<'f, 's> justact::Truth for Truth<'f, 's> {
    #[inline]
    fn value(&self) -> Option<bool> { self.value }
}

/// Wraps an [`Interpretation`] in order to implement [`Denotation`](justact::Denotation).
#[derive(Clone, Debug)]
pub struct Denotation<'f, 's> {
    /// A set of hard truths (including the effects, naively).
    truths:  HashMap<Atom<&'f str, &'s str>, Truth<'f, 's>>,
    /// An additional set of effects extracted from the `int`erpretation.
    effects: HashMap<Atom<&'f str, &'s str>, Effect<'f, 's>>,
}
impl<'f, 's> justact::Denotation for Denotation<'f, 's> {
    type Effect = Effect<'f, 's>;
    type Truth = Truth<'f, 's>;

    #[inline]
    fn truth_of(&self, fact: &<Self::Truth as justact::Identifiable>::Id) -> Option<bool> {
        if let Some(value) = self.truths.get(fact) { value.value } else { Some(false) }
    }
}
impl<'f, 's> justact::Set<Effect<'f, 's>> for Denotation<'f, 's> {
    type Error = Infallible;

    #[inline]
    fn get(&self, id: &<Effect<'f, 's> as justact::Identifiable>::Id) -> Result<Option<&Effect<'f, 's>>, Self::Error> { Ok(self.effects.get(id)) }

    #[inline]
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a Effect<'f, 's>>, Self::Error>
    where
        Effect<'f, 's>: 'a + justact::Identifiable,
    {
        Ok(self.effects.values())
    }
}
impl<'f, 's> justact::Set<Truth<'f, 's>> for Denotation<'f, 's> {
    type Error = Infallible;

    #[inline]
    fn get(&self, id: &<Truth<'f, 's> as justact::Identifiable>::Id) -> Result<Option<&Truth<'f, 's>>, Self::Error> { Ok(self.truths.get(id)) }

    #[inline]
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a Truth<'f, 's>>, Self::Error>
    where
        Truth<'f, 's>: 'a + justact::Identifiable,
    {
        Ok(self.truths.values())
    }
}
impl<'f, 's> From<Interpretation<'f, 's>> for Denotation<'f, 's> {
    #[inline]
    fn from(value: Interpretation<'f, 's>) -> Self { todo!() }
}



/// Wraps a [`Spec`] in order to implement [`Policy`](justact::Policy).
#[derive(Clone, Debug)]
pub struct Policy<'f, 's>(pub Spec<&'f str, &'s str>);
impl<'f, 's> Default for Policy<'f, 's> {
    #[inline]
    fn default() -> Self { Self(Spec { rules: Vec::new() }) }
}
impl<'f, 's> justact::Policy for Policy<'f, 's> {
    type Denotation = Denotation<'f, 's>;


    #[inline]
    fn is_valid(&self) -> bool {
        // Check whether error is true in the truths
        let atom: Atom<&'static str, &'static str> = Atom { ident: Ident { value: Span::new("<datalog::Policy::truths>", "error") }, args: None };
        if let Some(truth) = self.truths().truths.get(&atom) { truth.value == Some(true) } else { false }
    }

    #[inline]
    fn truths(&self) -> Self::Denotation {
        match self.0.alternating_fixpoint() {
            Ok(int) => int.into(),
            Err(_) => {
                // A critical failure occurred! Replace the denotation with one where error is true.
                let atom: Atom<&'static str, &'static str> =
                    Atom { ident: Ident { value: Span::new("<datalog::Policy::truths>", "error") }, args: None };
                Denotation { truths: HashMap::from([(atom.clone(), Truth { fact: atom, value: Some(true) })]), effects: HashMap::new() }
            },
        }
    }


    #[inline]
    fn compose(&self, other: Self) -> Self {
        let mut this = self.clone();
        this.compose_mut(other);
        this
    }

    #[inline]
    fn compose_mut(&mut self, other: Self) { self.0.rules.extend(other.0.rules); }
}



/// Represents the [`Extractor`] for Datalog's [`Spec`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Extractor;
impl justact::Extractor<str, str, String> for Extractor {
    type Policy<'m> = Policy<'m, 'm>;
    type Error<'m> = SyntaxError<'m>;


    #[inline]
    fn extract<'m, M: 'm + justact::Message<Id = str, AuthorId = str, Payload = String>>(
        msgs: &'m impl justact::Set<M>,
    ) -> Result<Self::Policy<'m>, Self::Error<'m>> {
        // Attempt to iterate over the messages
        let iter = msgs.iter().map_err(|err| SyntaxError::Iter { what: std::any::type_name::<M>(), err: Box::new(err) })?;

        // Parse the policy in the messages one-by-one
        let mut add_error: bool = false;
        let mut spec = Spec { rules: vec![] };
        for msg in iter {
            // Parse as UTF-8
            let snippet: &str = msg.payload();

            // Parse as Datalog
            let msg_spec: Spec<&'m str, &'m str> = match parse(msg.id(), snippet) {
                Ok(spec) => spec,
                Err(err) => return Err(SyntaxError::Datalog { err }),
            };

            // Check if there's any illegal rules
            if !add_error {
                'rules: for rule in &msg_spec.rules {
                    for cons in rule.consequences.values() {
                        // If a consequent begins with 'ctl-'...
                        if cons.ident.value.value().starts_with("ctl-") || cons.ident.value.value().starts_with("ctl_") {
                            // ...and its first argument is _not_ the author of the message...
                            if let Some(arg) = cons.args.iter().flat_map(|a| a.args.values().next()).next() {
                                if arg.ident().value.value() == msg.author_id() {
                                    continue;
                                } else {
                                    // ...then we derive error (it is not the author)
                                    add_error = true;
                                    break 'rules;
                                }
                            } else {
                                // ...then we derive error (there are no arguments)
                                add_error = true;
                                break 'rules;
                            }
                        }
                    }
                }
            }

            // OK, now we can add all the rules together
            spec.rules.extend(msg_spec.rules);
        }

        // If there were any illegal rules, inject error
        if add_error {
            // Build the list of consequences
            let mut consequences: Punctuated<Atom<&'m str, &'m str>, Comma<&'m str, &'m str>> = Punctuated::new();
            consequences.push_first(Atom { ident: Ident { value: Span::new("<datalog::Extractor::extract>", "error") }, args: None });

            // Then add the rule
            spec.rules.push(Rule { consequences, tail: None, dot: Dot { span: Span::new("<datalog::Extractor::extract>", ".") } })
        }

        // OK, return the spec
        Ok(Policy(spec))
    }
}
