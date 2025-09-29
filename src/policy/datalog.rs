//  DATALOG.rs
//    by Lut99
//
//  Created:
//    26 Nov 2024, 11:54:14
//  Last edited:
//    21 Jan 2025, 15:06:32
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements JustAct traits for the [`datalog`]-crate.
//

use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::ops::{Deref, DerefMut};

use datalog::ast::{Atom, AtomArg, AtomArgs, Comma, Dot, Ident, Parens, Punctuated, Rule, Span, Spec, punct};
use datalog::interpreter::interpretation::Interpretation;
use datalog::parser::parse;
use error_trace::toplevel;
use thiserror::Error;
mod justact {
    pub use ::justact::auxillary::{Affectored, Identifiable};
    pub use ::justact::collections::map::Map;
    pub use ::justact::collections::set::Set;
    pub use ::justact::messages::Message;
    pub use ::justact::policies::{Denotation, Effect, Extractor, Policy};
}


/***** ERRORS *****/
/// Defines errors that may occur when [extracting](Extractor::extract()) policy.
#[derive(Debug, Error)]
pub enum SyntaxError<'m> {
    #[error("{}", toplevel!(("Failed to parse the input as valid Datalog"), err))]
    Datalog { err: datalog::parser::Error<&'m str, &'m str> },
    #[error("{}", toplevel!(("Failed to iterate over messages in {what}"), &**err))]
    Iter { what: &'static str, err: Box<dyn 'm + Error> },
}





/***** LIBRARY *****/
/// Wraps a Datalog fact of a VERY particular shape as an [`Effect`](justact::Effect).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Effect<'f, 's> {
    /// The truth wrapped by this effect.
    pub fact:     Atom<&'f str, &'s str>,
    /// The identifier of the affector.
    pub affector: Ident<&'f str, &'s str>,
}
impl<'f, 's> justact::Affectored for Effect<'f, 's> {
    type AffectorId = Ident<&'f str, &'s str>;

    #[inline]
    fn affector_id(&self) -> &Self::AffectorId { &self.affector }
}
impl<'f, 's> justact::Effect for Effect<'f, 's> {
    type Fact = Atom<&'f str, &'s str>;

    #[inline]
    fn fact(&self) -> &Self::Fact { &self.fact }
}
impl<'f, 's> justact::Identifiable for Effect<'f, 's> {
    type Id = Atom<&'f str, &'s str>;

    #[inline]
    fn id(&self) -> &Self::Id { &self.fact }
}

/// Wraps an [`Interpretation`] in order to implement [`Denotation`](justact::Denotation).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Denotation<'f, 's> {
    /// A set of hard truths (including the effects, naively).
    truths:  HashMap<Atom<&'f str, &'s str>, Option<bool>>,
    /// An additional set of effects extracted from the `int`erpretation.
    effects: HashMap<Atom<&'f str, &'s str>, Effect<'f, 's>>,
}
impl<'f, 's> Default for Denotation<'f, 's> {
    #[inline]
    fn default() -> Self { Self { truths: HashMap::new(), effects: HashMap::new() } }
}
impl<'f, 's> Denotation<'f, 's> {
    /// Creates a new Denotation from a Datalog [`Interpretation`].
    ///
    /// # Arguments
    /// - `int`: The [`Interpretation`] to build this Denotation from.
    /// - `pat`: An [`Atom`] that describes a pattern for recognizing effets.
    /// - `affector`: An [`AtomArg`] that describes how to extract the affector from the effect.
    ///
    /// # Returns
    /// A new Denotation that is JustAct^{TM} compliant.
    #[inline]
    pub fn from_interpretation(int: Interpretation<'f, 's>, pat: Atom<&'f str, &'s str>, affector: AtomArg<&'f str, &'s str>) -> Self {
        let mut truths: HashMap<Atom<&'f str, &'s str>, Option<bool>> = HashMap::new();
        let mut effects: HashMap<Atom<&'f str, &'s str>, Effect<'f, 's>> = HashMap::new();
        for (fact, value) in int.into_iter() {
            // See if the fact matches the pattern
            if pat.ident == fact.ident
                && pat.args.as_ref().map(|a| a.args.len()).unwrap_or(0) == fact.args.as_ref().map(|a| a.args.len()).unwrap_or(0)
            {
                // The identifier and arity matches; check if every non-variable argument in the pattern matches the fact
                let mut is_effect: bool = true;
                for (fact_arg, pat_arg) in pat.args.iter().flat_map(|a| a.args.values()).zip(fact.args.iter().flat_map(|a| a.args.values())) {
                    if matches!(pat_arg, AtomArg::Atom(_)) && pat_arg == fact_arg {
                        // It's not an effect :(
                        is_effect = false;
                        break;
                    }
                }
                // Insert it if the argument of the pattern matched
                if is_effect {
                    // Extract the affector
                    let affector: &Ident<&'f str, &'s str> = match &affector {
                        // If it's an atom, just return that
                        AtomArg::Atom(a) => a,
                        // If it's a variable, find it in the effect
                        AtomArg::Var(v) => {
                            match pat.args.iter().flat_map(|a| a.args.values()).zip(fact.args.iter().flat_map(|a| a.args.values())).find_map(
                                |(pat_arg, fact_arg)| {
                                    if let AtomArg::Var(arg) = pat_arg { if arg == v { Some(fact_arg.ident()) } else { None } } else { None }
                                },
                            ) {
                                Some(ident) => ident,
                                None => panic!("Did not find affector variable {:?} in effect {:?}", v.value.value(), fact),
                            }
                        },
                    };

                    // Insert it!
                    effects.insert(fact.clone(), Effect { fact: fact.clone(), affector: affector.clone() });
                }
            }

            // Always add the truth as such
            truths.insert(fact, value);
        }

        // OK, return the denotation!
        Self { truths, effects }
    }
}
impl<'f, 's> justact::Denotation for Denotation<'f, 's> {
    type Effect = Effect<'f, 's>;
    type Fact = Atom<&'f str, &'s str>;

    #[inline]
    fn truth_of(&self, fact: &Self::Fact) -> Option<bool> { self.truths.get(fact).cloned().unwrap_or(Some(false)) }
}
impl<'f, 's> justact::Map<Effect<'f, 's>> for Denotation<'f, 's> {
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

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { Ok(self.effects.len()) }
}
impl<'f, 's> justact::Set<Atom<&'f str, &'s str>> for Denotation<'f, 's> {
    type Error = Infallible;

    #[inline]
    fn get(&self, elem: &Atom<&'f str, &'s str>) -> Result<Option<&Atom<&'f str, &'s str>>, Self::Error> {
        Ok(self.truths.get_key_value(elem).map(|(k, _)| k))
    }

    #[inline]
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a Atom<&'f str, &'s str>>, Self::Error>
    where
        Atom<&'f str, &'s str>: 'a,
    {
        Ok(self.truths.keys())
    }

    #[inline]
    fn len(&self) -> Result<usize, Self::Error> { Ok(self.truths.len()) }
}



/// Wraps a [`Spec`] in order to implement [`Policy`](justact::Policy).
#[derive(Clone, Debug)]
pub struct Policy<'f, 's> {
    /// The pattern to match effects.
    pat:      Atom<&'f str, &'s str>,
    /// How to find the affector from effects.
    affector: AtomArg<&'f str, &'s str>,
    /// The spec we wrap with actual policy.
    spec:     Spec<&'f str, &'s str>,
}
impl<'f, 's> Default for Policy<'f, 's> {
    #[inline]
    fn default() -> Self {
        Self {
            pat:      Atom {
                ident: Ident { value: Span::new(("<Policy::default()>", "effect")) },
                args:  Some(AtomArgs {
                    paren_tokens: Parens { open: Span::new(("<Policy::default()>", "(")), close: Span::new(("<Policy::default()>", ")")) },
                    args: punct![
                        AtomArg::Var(Ident { value: Span::new(("<Policy::default()>", "A")) }),
                        Comma { span: Span::new(("<Policy::default()>", ",")) },
                        AtomArg::Var(Ident { value: Span::new(("<Policy::default()>", "E")) })
                    ],
                }),
            },
            affector: AtomArg::Var(Ident { value: Span::new(("<Policy::default()>", "A")) }),
            spec:     Spec { rules: Vec::new() },
        }
    }
}
impl<'f, 's> Policy<'f, 's> {
    /// Updates the pattern that matches Datalog atoms to match effects.
    ///
    /// By default, any Datalog atom of the shape 'effect(Affector, Effect)` is seen as an effect.
    /// But this may not always be desired; and as such, another pattern can be given.
    ///
    /// The pattern can use variables as wildcards. Then, the `affector` can optionally use one of
    /// those to communicate one of those variables encodes the affector.
    ///
    /// # Arguments
    /// - `pat`: The pattern used to match effects.
    /// - `affector`: What affector to provide for effects.
    #[inline]
    pub fn update_effect_pattern(&mut self, pat: Atom<&'f str, &'s str>, affector: AtomArg<&'f str, &'s str>) {
        self.pat = pat;
        self.affector = affector;
    }

    /// Returns the specification.
    ///
    /// # Returns
    /// A reference to the internal [`Spec`].
    #[inline]
    pub fn spec(&self) -> &Spec<&'f str, &'s str> { &self.spec }
    /// Returns the specification mutably.
    ///
    /// # Returns
    /// A mutable reference to the internal [`Spec`].
    #[inline]
    pub fn spec_mut(&mut self) -> &mut Spec<&'f str, &'s str> { &mut self.spec }
    /// Returns the specification by consuming this Policy.
    ///
    /// # Returns
    /// The internal [`Spec`].
    #[inline]
    pub fn into_spec(self) -> Spec<&'f str, &'s str> { self.spec }
}
impl<'f, 's> justact::Policy for Policy<'f, 's> {
    type Denotation = Denotation<'f, 's>;


    #[inline]
    fn is_valid(&self) -> bool {
        // Check whether error is true in the truths
        let atom: Atom<&'static str, &'static str> = Atom { ident: Ident { value: Span::new(("<datalog::Policy::truths>", "error")) }, args: None };
        if let Some(value) = self.truths().truths.get(&atom) { value != &Some(true) } else { true }
    }

    #[inline]
    fn truths(&self) -> Self::Denotation {
        Denotation::from_interpretation(self.spec.alternating_fixpoint(), self.pat.clone(), self.affector.clone())
    }


    #[inline]
    fn compose(&self, other: Self) -> Self {
        let mut this = self.clone();
        this.compose_mut(other);
        this
    }

    #[inline]
    fn compose_mut(&mut self, other: Self) { self.spec.rules.extend(other.spec.rules); }
}
impl<'f, 's> Deref for Policy<'f, 's> {
    type Target = Spec<&'f str, &'s str>;
    #[inline]
    fn deref(&self) -> &Self::Target { &self.spec }
}
impl<'f, 's> DerefMut for Policy<'f, 's> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.spec }
}



/// Represents the [`Extractor`] for Datalog's [`Spec`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Extractor;
impl justact::Extractor<str, str, str> for Extractor {
    type Policy<'m> = Policy<'m, 'm>;
    type Error<'m> = SyntaxError<'m>;


    #[inline]
    fn extract<'m, 'm2: 'm, M: 'm2 + justact::Message<Id = str, AuthorId = str, Payload = str>>(
        &self,
        msgs: &'m impl justact::Map<M>,
    ) -> Result<Self::Policy<'m>, Self::Error<'m>> {
        // Attempt to iterate over the messages
        let iter = msgs.iter().map_err(|err| SyntaxError::Iter { what: std::any::type_name::<M>(), err: Box::new(err) })?;

        // Parse the policy in the messages one-by-one
        let mut add_error: bool = false;
        let mut policy = Policy::default();
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
            policy.spec.rules.extend(msg_spec.rules);
        }

        // If there were any illegal rules, inject error
        if add_error {
            // Build the list of consequents
            let mut consequents: Punctuated<Atom<&'m str, &'m str>, Comma<&'m str, &'m str>> = Punctuated::new();
            consequents.push_first(Atom { ident: Ident { value: Span::new(("<datalog::Extractor::extract>", "error")) }, args: None });

            // Then add the rule
            policy.spec.rules.push(Rule { consequents, tail: None, dot: Dot { span: Span::new(("<datalog::Extractor::extract>", ".")) } })
        }

        // OK, return the spec
        Ok(policy)
    }
}





/***** TESTS *****/
#[cfg(all(test, feature = "lang-macros"))]
mod tests {
    use std::collections::HashMap;
    use std::convert::Infallible;

    use datalog::ast::{Atom, AtomArg, AtomArgs, Comma, Ident, Parens, Span, datalog, punct};

    use super::{Denotation, Effect, Extractor, Policy};
    mod justact {
        pub use ::justact::auxillary::Authored;
        pub use ::justact::messages::MessageSet;

        pub use super::super::justact::*;
    }


    /// Generates the effect pattern.
    fn make_effect(actor: &'static str, effect: &'static str) -> Atom<&'static str, &'static str> {
        Atom {
            ident: Ident { value: Span::new(("<make_effect>", "effect")) },
            args:  Some(AtomArgs {
                paren_tokens: Parens { open: Span::new(("<make_effect>", "(")), close: Span::new(("<make_effect>", ")")) },
                args: punct![
                    if actor.chars().next().map(char::is_uppercase).unwrap_or(false) {
                        AtomArg::Var(Ident { value: Span::new(("<make_effect>", actor)) })
                    } else {
                        AtomArg::Atom(Ident { value: Span::new(("<make_effect>", actor)) })
                    },
                    Comma { span: Span::new(("<make_effect>", ",")) },
                    if effect.chars().next().map(char::is_uppercase).unwrap_or(false) {
                        AtomArg::Var(Ident { value: Span::new(("<make_effect>", effect)) })
                    } else {
                        AtomArg::Atom(Ident { value: Span::new(("<make_effect>", effect)) })
                    }
                ],
            }),
        }
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
    impl justact::Map<Self> for Message {
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
        #[inline]
        fn len(&self) -> Result<usize, Self::Error> { Ok(1) }
    }


    #[test]
    fn test_extract_policy_single() {
        let msg = Message { id: "A".into(), author_id: "Amy".into(), payload: "foo. bar :- baz(A).".into() };
        let pol = <Extractor as justact::Extractor<str, str, str>>::extract(&Extractor, &msg).unwrap();
        assert_eq!(pol.spec, datalog!( foo. bar :- baz(A). ));
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
        assert!(pol.spec == datalog!( foo. bar :- baz(A). ) || pol.spec == datalog!( bar :- baz(A). foo. ));
    }

    #[test]
    fn test_is_valid() {
        let mut pol = Policy::default();
        pol.spec = datalog!( foo. bar :- baz(A). );
        assert!(<Policy as justact::Policy>::is_valid(&pol));
    }
    #[test]
    fn test_is_not_valid() {
        let mut pol = Policy::default();
        pol.spec = datalog!( error :- foo. foo. );
        assert!(!<Policy as justact::Policy>::is_valid(&pol));
    }

    #[test]
    fn test_truths() {
        let mut pol = Policy::default();
        pol.spec = datalog!( foo. bar :- baz(A). );
        let den = <Policy as justact::Policy>::truths(&pol);
        assert_eq!(den, Denotation {
            truths:  [
                (Atom { ident: Ident { value: Span::new("<test_truths>", "foo") }, args: None }, Some(true)),
                (Atom { ident: Ident { value: Span::new("<test_truths>", "bar") }, args: None }, Some(false)),
            ]
            .into_iter()
            .map(|(a, v)| (a, v))
            .collect(),
            effects: HashMap::new(),
        })
    }
    #[test]
    fn test_effects() {
        let mut pol = Policy::default();
        pol.spec = datalog!( effect(amy, read). effect(amy, write) :- baz(A). );
        let den = <Policy as justact::Policy>::truths(&pol);
        assert_eq!(den, Denotation {
            truths:  [make_effect("amy", "read")].into_iter().map(|a| (a, Some(true))).collect(),
            effects: [make_effect("amy", "read")]
                .into_iter()
                .map(|a| (a.clone(), Effect { fact: a, affector: Ident { value: Span::new("<test_effects>", "amy") } }))
                .collect(),
        })
    }
}
