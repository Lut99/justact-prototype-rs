//  DATALOG.rs
//    by Lut99
//
//  Created:
//    26 Nov 2024, 11:54:14
//  Last edited:
//    19 Dec 2024, 12:17:24
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements JustAct traits for the [`datalog`]-crate.
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
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Denotation<'f, 's> {
    /// A set of hard truths (including the effects, naively).
    truths:  HashMap<Atom<&'f str, &'s str>, Truth<'f, 's>>,
    /// An additional set of effects extracted from the `int`erpretation.
    effects: HashMap<Atom<&'f str, &'s str>, Effect<'f, 's>>,
}
impl<'f, 's> Default for Denotation<'f, 's> {
    #[inline]
    fn default() -> Self { Self { truths: HashMap::new(), effects: HashMap::new() } }
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
    fn from(value: Interpretation<'f, 's>) -> Self {
        let mut den = Denotation::default();
        for (fact, value) in value.into_iter() {
            // See if this happens to be an effect
            if fact.ident.value.value() == "effect" {
                if let Some(args) = &fact.args {
                    if args.args.len() == 2 {
                        // It is! Inject it
                        den.effects.insert(fact.clone(), Effect(Truth { fact: fact.clone(), value }));
                    }
                }
            }

            // Always implement the truth
            den.truths.insert(fact.clone(), Truth { fact, value });
        }
        den
    }
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
        if let Some(truth) = self.truths().truths.get(&atom) { truth.value != Some(true) } else { true }
    }

    #[inline]
    fn truths(&self) -> Self::Denotation { self.0.alternating_fixpoint().into() }


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
impl justact::Extractor<str, str, str> for Extractor {
    type Policy<'m> = Policy<'m, 'm>;
    type Error<'m> = SyntaxError<'m>;


    #[inline]
    fn extract<'m, M: 'm + justact::Message<Id = str, AuthorId = str, Payload = str>>(
        &self,
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
                    for cons in rule.consequents.values() {
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
            // Build the list of consequents
            let mut consequents: Punctuated<Atom<&'m str, &'m str>, Comma<&'m str, &'m str>> = Punctuated::new();
            consequents.push_first(Atom { ident: Ident { value: Span::new("<datalog::Extractor::extract>", "error") }, args: None });

            // Then add the rule
            spec.rules.push(Rule { consequents, tail: None, dot: Dot { span: Span::new("<datalog::Extractor::extract>", ".") } })
        }

        // OK, return the spec
        Ok(Policy(spec))
    }
}





/***** TESTS *****/
#[cfg(all(test, feature = "lang-macros"))]
mod tests {
    use std::collections::HashMap;
    use std::convert::Infallible;

    use datalog::ast::{Atom, AtomArg, AtomArgs, Comma, Ident, Parens, Span, datalog, punct};

    use super::{Denotation, Effect, Extractor, Policy, Truth};
    mod justact {
        pub use ::justact::auxillary::Authored;
        pub use ::justact::messages::MessageSet;

        pub use super::super::justact::*;
    }


    /// Implements a test message
    struct Message {
        id: String,
        author_id: String,
        payload: String,
    }
    impl justact::Authored for Message {
        type AuthorId = str;
        #[inline]
        fn author_id(&self) -> &Self::AuthorId { &self.author_id }
    }
    impl justact::Identifiable for Message {
        type Id = str;
        #[inline]
        fn id(&self) -> &Self::Id { &self.id }
    }
    impl justact::Message for Message {
        type Payload = str;
        #[inline]
        fn payload(&self) -> &Self::Payload { &self.payload }
    }
    impl justact::Set<Self> for Message {
        type Error = Infallible;
        #[inline]
        fn get(&self, id: &<Self as justact::Identifiable>::Id) -> Result<Option<&Self>, Self::Error>
        where
            Self: justact::Identifiable,
        {
            if &self.id == id { Ok(Some(self)) } else { Ok(None) }
        }
        #[inline]
        fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s Self>, Self::Error>
        where
            Self: 's + justact::Identifiable,
        {
            Ok(Some(self).into_iter())
        }
    }


    #[test]
    fn test_extract_policy_single() {
        let msg = Message { id: "A".into(), author_id: "Amy".into(), payload: "foo. bar :- baz(A).".into() };
        let pol = <Extractor as justact::Extractor<str, str, str>>::extract(&Extractor, &msg).unwrap();
        assert_eq!(pol.0, datalog!( foo. bar :- baz(A). ));
    }
    #[test]
    fn test_extract_policy_multi() {
        // Construct a set of messages
        let msg1 = Message { id: "A".into(), author_id: "Amy".into(), payload: "foo.".into() };
        let msg2 = Message { id: "B".into(), author_id: "Bob".into(), payload: "bar :- baz(A).".into() };
        let msgs = justact::MessageSet::from([msg1, msg2]);

        // Extract the policy from it
        let pol = <Extractor as justact::Extractor<str, str, str>>::extract(&Extractor, &msgs).unwrap();
        // NOTE: MessageSet collects messages unordered, so the rules may be in any order
        assert!(pol.0 == datalog!( foo. bar :- baz(A). ) || pol.0 == datalog!( bar :- baz(A). foo. ));
    }

    #[test]
    fn test_is_valid() {
        let pol = Policy(datalog!( foo. bar :- baz(A). ));
        assert!(<Policy as justact::Policy>::is_valid(&pol));
    }
    #[test]
    fn test_is_not_valid() {
        let pol = Policy(datalog!( error :- foo. foo. ));
        assert!(!<Policy as justact::Policy>::is_valid(&pol));
    }

    #[test]
    fn test_truths() {
        let pol = Policy(datalog!( foo. bar :- baz(A). ));
        let den = <Policy as justact::Policy>::truths(&pol);
        assert_eq!(den, Denotation {
            truths:  [
                (Atom { ident: Ident { value: Span::new("<test_truths>", "foo") }, args: None }, Some(true)),
                (Atom { ident: Ident { value: Span::new("<test_truths>", "bar") }, args: None }, Some(false)),
            ]
            .into_iter()
            .map(|(a, v)| (a.clone(), Truth { fact: a, value: v }))
            .collect(),
            effects: HashMap::new(),
        })
    }
    #[test]
    fn test_effects() {
        fn make_effect(actor: &'static str, effect: &'static str) -> Atom<&'static str, &'static str> {
            Atom {
                ident: Ident { value: Span::new("<test_truths>", "effect") },
                args:  Some(AtomArgs {
                    paren_tokens: Parens { open: Span::new("<test_truths>", "("), close: Span::new("<test_thruths>", ")") },
                    args: punct![
                        v => AtomArg::Atom(Ident { value: Span::new("<test_effects>", actor) }),
                        p => Comma { span: Span::new("<test_effects>", ",") },
                        v => AtomArg::Atom(Ident { value: Span::new("<test_effects>", effect) })
                    ],
                }),
            }
        }

        let pol = Policy(datalog!( effect(amy, read). effect(amy, write) :- baz(A). ));
        let den = <Policy as justact::Policy>::truths(&pol);
        println!("{den:#?}");
        assert_eq!(den, Denotation {
            truths:  [(make_effect("amy", "read"), Some(true))].into_iter().map(|(a, v)| (a.clone(), Truth { fact: a, value: v })).collect(),
            effects: [(make_effect("amy", "read"), Some(true))].into_iter().map(|(a, v)| (a.clone(), Effect(Truth { fact: a, value: v }))).collect(),
        })
    }
}
