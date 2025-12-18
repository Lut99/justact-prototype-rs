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

use datalog::interpreter::KnowledgeBase;
use datalog::parser::parse;
use datalog::{ast, ir};
use error_trace::toplevel;
use thiserror::Error;

use super::{PolicyDeserialize, PolicySerialize};
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
    Datalog { err: datalog::parser::Error<'m, (&'m str, &'m str)> },
    #[error("{}", toplevel!(("Failed to iterate over messages in {what}"), &**err))]
    Iter { what: &'static str, err: Box<dyn 'm + Error> },
}





/***** LIBRARY *****/
/// Wraps a Datalog fact of a VERY particular shape as an [`Effect`](justact::Effect).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Effect<'f, 's> {
    /// The truth wrapped by this effect.
    pub fact:     ir::GroundAtom<(&'f str, &'s str)>,
    /// The identifier of the affector.
    pub affector: ir::Ident<(&'f str, &'s str)>,
}
impl<'f, 's> justact::Affectored for Effect<'f, 's> {
    type AffectorId = ir::Ident<(&'f str, &'s str)>;

    #[inline]
    fn affector_id(&self) -> &Self::AffectorId { &self.affector }
}
impl<'f, 's> justact::Effect for Effect<'f, 's> {
    type Fact = ir::GroundAtom<(&'f str, &'s str)>;

    #[inline]
    fn fact(&self) -> &Self::Fact { &self.fact }
}
impl<'f, 's> justact::Identifiable for Effect<'f, 's> {
    type Id = ir::GroundAtom<(&'f str, &'s str)>;

    #[inline]
    fn id(&self) -> &Self::Id { &self.fact }
}

/// Wraps an [`Interpretation`] in order to implement [`Denotation`](justact::Denotation).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Denotation<'f, 's> {
    /// A set of hard truths (including the effects, naively).
    truths:  HashMap<ir::GroundAtom<(&'f str, &'s str)>, Option<bool>>,
    /// An additional set of effects extracted from the `int`erpretation.
    effects: HashMap<ir::GroundAtom<(&'f str, &'s str)>, Effect<'f, 's>>,
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
    /// - `pat`: An [`Atom`] that describes a pattern for recognizing effects.
    /// - `affector`: An [`Atom`] that describes how to extract the affector from the effect.
    ///
    /// # Returns
    /// A new Denotation that is JustAct^{TM} compliant.
    #[inline]
    pub fn from_interpretation(
        int: KnowledgeBase<(&'f str, &'s str)>,
        pat: ir::Atom<(&'f str, &'s str)>,
        affector: ir::Ident<(&'f str, &'s str)>,
    ) -> Self {
        fn matched_by<S: Clone>(atom: &ir::GroundAtom<S>, pat: &ir::Atom<S>, affector: &ir::Ident<S>) -> Option<ir::Ident<S>> {
            match pat {
                // Check if arity matches
                ir::Atom::Fact(ir::Fact { ident, args }) if &atom.ident == ident && atom.args.len() == args.len() => {
                    // Check if the pattern checks out
                    let mut res: Option<ir::Ident<S>> = None;
                    for (atom, pat) in atom.args.iter().zip(args.iter()) {
                        if let Some(aff) = matched_by(atom, pat, affector) {
                            if res.is_none() {
                                res = Some(aff);
                            } else {
                                panic!("Duplicate affector!");
                            }
                        }
                    }
                    res
                },
                ir::Atom::Fact(_) => None,

                // Variables always match, but not necessarily the affector
                ir::Atom::Var(var) if var == affector => {
                    // We can only do something if the fact is an ident
                    if !atom.args.is_empty() {
                        panic!("Can only match affector with identifier, not nested thing!");
                    }
                    Some(atom.ident.clone())
                },
                ir::Atom::Var(_) => None,
            }
        }


        let mut truths: HashMap<ir::GroundAtom<(&'f str, &'s str)>, Option<bool>> = HashMap::new();
        let mut effects: HashMap<ir::GroundAtom<(&'f str, &'s str)>, Effect<'f, 's>> = HashMap::new();
        for fact in int.truths() {
            // See if the fact matches the pattern
            if let Some(affector) = matched_by(&fact, &pat, &affector) {
                // Insert it!
                effects.insert(fact.clone(), Effect { fact: fact.clone(), affector });
            }

            // Always add the truth as such
            truths.insert(fact.clone(), Some(true));
        }

        // OK, return the denotation!
        Self { truths, effects }
    }
}
impl<'f, 's> justact::Denotation for Denotation<'f, 's> {
    type Effect = Effect<'f, 's>;
    type Fact = ir::GroundAtom<(&'f str, &'s str)>;

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
impl<'f, 's> justact::Set<ir::GroundAtom<(&'f str, &'s str)>> for Denotation<'f, 's> {
    type Error = Infallible;

    #[inline]
    fn get(&self, elem: &ir::GroundAtom<(&'f str, &'s str)>) -> Result<Option<&ir::GroundAtom<(&'f str, &'s str)>>, Self::Error> {
        Ok(self.truths.get_key_value(elem).map(|(k, _)| k))
    }

    #[inline]
    fn iter<'a>(&'a self) -> Result<impl Iterator<Item = &'a ir::GroundAtom<(&'f str, &'s str)>>, Self::Error>
    where
        ir::GroundAtom<(&'f str, &'s str)>: 'a,
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
    pat:      ir::Atom<(&'f str, &'s str)>,
    /// How to find the affector from effects.
    affector: ir::Ident<(&'f str, &'s str)>,
    /// The spec we wrap with actual policy.
    spec:     ir::Spec<ir::Atom<(&'f str, &'s str)>>,
}
impl<'f, 's> Default for Policy<'f, 's> {
    #[inline]
    fn default() -> Self {
        Self {
            pat:      ir::Atom::Fact(ir::Fact {
                ident: ir::Ident::new("effect".into(), None),
                args:  vec![ir::Atom::Var(ir::Ident::new("A".into(), None)), ir::Atom::Var(ir::Ident::new("E".into(), None))],
            }),
            affector: ir::Ident::new("A".into(), None),
            spec:     ir::Spec { rules: Vec::new() },
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
    pub fn update_effect_pattern(&mut self, pat: ir::Atom<(&'f str, &'s str)>, affector: ir::Ident<(&'f str, &'s str)>) {
        self.pat = pat;
        self.affector = affector;
    }

    /// Returns the specification.
    ///
    /// # Returns
    /// A reference to the internal [`Spec`].
    #[inline]
    pub fn spec(&self) -> &ir::Spec<ir::Atom<(&'f str, &'s str)>> { &self.spec }
    /// Returns the specification mutably.
    ///
    /// # Returns
    /// A mutable reference to the internal [`Spec`].
    #[inline]
    pub fn spec_mut(&mut self) -> &mut ir::Spec<ir::Atom<(&'f str, &'s str)>> { &mut self.spec }
    /// Returns the specification by consuming this Policy.
    ///
    /// # Returns
    /// The internal [`Spec`].
    #[inline]
    pub fn into_spec(self) -> ir::Spec<ir::Atom<(&'f str, &'s str)>> { self.spec }
}
impl<'f, 's> justact::Policy for Policy<'f, 's> {
    type Denotation = Denotation<'f, 's>;


    #[inline]
    fn is_valid(&self) -> bool {
        // Check whether error is true in the truths
        let atom: ir::GroundAtom<(&'static str, &'static str)> = ir::GroundAtom { ident: ir::Ident::new("error".into(), None), args: Vec::new() };
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
    type Target = ir::Spec<ir::Atom<(&'f str, &'s str)>>;
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
impl<'m> justact::Extractor<str, ast::Spec<(&'m str, &'m str)>> for Extractor {
    type Policy<'a> = Policy<'m, 'm>;
    type Error<'a> = SyntaxError<'m>;


    #[inline]
    fn extract<'a, M: justact::Message<AuthorId = str, Payload = ast::Spec<(&'m str, &'m str)>>>(
        &self,
        msgs: &'a impl justact::Set<M>,
    ) -> Result<Self::Policy<'m>, Self::Error<'m>> {
        // Attempt to iterate over the messages
        let iter = msgs.iter().map_err(|err| SyntaxError::Iter { what: std::any::type_name::<M>(), err: Box::new(err) })?;

        // Parse the policy in the messages one-by-one
        let mut add_error: bool = false;
        let mut policy = Policy::default();
        for msg in iter {
            // Get the Datalog
            let msg_spec: &ast::Spec<(&'m str, &'m str)> = msg.payload();

            // Check if there's any illegal rules
            if !add_error {
                'rules: for rule in &msg_spec.rules {
                    for cons in rule.consequents.values() {
                        let ast::Atom::Fact(cons) = cons else { continue };

                        // If a consequent begins with 'ctl-'...
                        if cons.ident.value.starts_with("ctl-") || cons.ident.value.starts_with("ctl_") {
                            // ...and its first argument is _not_ the author of the message...
                            if let Some(arg) = cons.args.iter().flat_map(|a| a.args.values().next()).next() {
                                match arg {
                                    ast::Atom::Fact(f) => {
                                        if f.args.as_ref().map(|a| a.args.len()).unwrap_or(0) == 0 {
                                            if f.ident.value != msg.author_id() {
                                                // ...then we derive error (it is not the author)
                                                add_error = true;
                                                break 'rules;
                                            }
                                        } else {
                                            // ...then we derive error (it is not flat)
                                            add_error = true;
                                            break 'rules;
                                        }
                                    },
                                    ast::Atom::Var(_) => {
                                        // ...then we derive error (it is a variable)
                                        add_error = true;
                                        break 'rules;
                                    },
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
            policy.spec.rules.reserve(msg_spec.rules.len());
            for rule in &msg_spec.rules {
                // Attempt to compile it
                match rule.compile() {
                    Ok(rule) => policy.spec.rules.push(rule),
                    Err(_) => {
                        // Replace the policy set with error, then quit
                        policy.spec.rules = vec![ir::Rule {
                            consequents:     vec![ir::Atom::Fact(ir::Fact { ident: ir::Ident::new("error".into(), None), args: Vec::new() })],
                            pos_antecedents: Vec::new(),
                            neg_antecedents: Vec::new(),
                        }];
                        break;
                    },
                }
            }
        }

        // If there were any illegal rules, inject error
        if add_error {
            policy.spec.rules.push(ir::Rule {
                consequents:     vec![ir::Atom::Fact(ir::Fact { ident: ir::Ident::new("error".into(), None), args: Vec::new() })],
                pos_antecedents: Vec::new(),
                neg_antecedents: Vec::new(),
            });
        }

        // OK, return the spec
        Ok(policy)
    }
}



impl<'a> PolicySerialize for ast::Spec<(&'a str, &'a str)> {
    #[inline]
    fn serialize(&self) -> String { format!("{self}") }
}
impl<'a> PolicyDeserialize<'a> for ast::Spec<(&'static str, &'a str)> {
    type Error = datalog::parser::Error<'a, (&'static str, &'a str)>;

    #[inline]
    fn deserialize(raw: &'a str) -> Result<Self::Owned, Self::Error> { Ok(parse(("<raw>", raw))?) }
}





/***** TESTS *****/
#[cfg(all(test, feature = "lang-macros"))]
mod tests {
    use std::collections::HashMap;
    use std::convert::Infallible;

    use datalog::ast::{Spec, datalog};
    use datalog::ir::{GroundAtom, Ident};

    use super::{Denotation, Effect, Extractor, Policy};
    mod justact {
        pub use ::justact::auxillary::Authored;
        pub use ::justact::messages::MessageSet;

        pub use super::super::justact::*;
    }


    /// Generates the effect pattern.
    fn make_effect(actor: &'static str, effect: &'static str) -> GroundAtom<(&'static str, &'static str)> {
        GroundAtom {
            ident: Ident::new("effect".into(), None),
            args:  vec![GroundAtom { ident: Ident::new(actor.into(), None), args: Vec::new() }, GroundAtom {
                ident: Ident::new(effect.into(), None),
                args:  Vec::new(),
            }],
        }
    }

    /// Implements a test message
    #[derive(Eq, Hash, PartialEq)]
    struct Message {
        author_id: String,
        payload:   Spec<(&'static str, &'static str)>,
    }
    impl justact::Authored for Message {
        type AuthorId = str;
        #[inline]
        fn author_id(&self) -> &Self::AuthorId { &self.author_id }
    }
    impl justact::Message for Message {
        type Payload = Spec<(&'static str, &'static str)>;

        #[inline]
        fn payload(&self) -> &Self::Payload { &self.payload }
    }
    impl justact::Set<Self> for Message {
        type Error = Infallible;
        #[inline]
        fn get(&self, elem: &Self) -> Result<Option<&Self>, Self::Error> { if self == elem { Ok(Some(self)) } else { Ok(None) } }
        #[inline]
        fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s Self>, Self::Error>
        where
            Self: 's,
        {
            Ok(Some(self).into_iter())
        }
        #[inline]
        fn len(&self) -> Result<usize, Self::Error> { Ok(1) }
    }


    #[test]
    fn test_extract_policy_single() {
        let msg = Message { author_id: "Amy".into(), payload: datalog!(foo. bar :- baz(A).) };
        let pol = <Extractor as justact::Extractor<str, Spec<(&str, &str)>>>::extract(&Extractor, &msg).unwrap();
        assert_eq!(pol.spec, datalog!( foo. bar :- baz(A). ).compile().unwrap());
    }
    #[test]
    fn test_extract_policy_multi() {
        // Construct a set of messages
        let msg1 = Message { author_id: "Amy".into(), payload: datalog!(foo.) };
        let msg2 = Message { author_id: "Bob".into(), payload: datalog!(bar :- baz(A).) };
        let msgs = justact::MessageSet::from_iter([msg1, msg2]);

        // Extract the policy from it
        // NOTE: MessageSet collects messages unordered, so we'll have to sort them to get a deterministic answer
        let mut pol = <Extractor as justact::Extractor<str, Spec<(&str, &str)>>>::extract(&Extractor, &msgs).unwrap();
        pol.rules.sort_by(|r1, r2| r1.to_string().cmp(&r2.to_string()));

        // NOTE: MessageSet collects messages unordered, so the rules may be in any order
        assert_eq!(pol.spec, datalog!( bar :- baz(A). foo. ).compile().unwrap());
    }

    #[test]
    fn test_is_valid() {
        let mut pol = Policy::default();
        pol.spec = datalog!( foo. bar :- baz(A). ).compile().unwrap();
        assert!(<Policy as justact::Policy>::is_valid(&pol));
    }
    #[test]
    fn test_is_not_valid() {
        let mut pol = Policy::default();
        pol.spec = datalog!( error :- foo. foo. ).compile().unwrap();
        assert!(!<Policy as justact::Policy>::is_valid(&pol));
    }

    #[test]
    fn test_truths() {
        let mut pol = Policy::default();
        pol.spec = datalog!( foo. bar :- baz(A). ).compile().unwrap();
        let den = <Policy as justact::Policy>::truths(&pol);
        assert_eq!(den, Denotation {
            truths:  [(GroundAtom { ident: Ident::new("foo".into(), None), args: Vec::new() }, Some(true)),]
                .into_iter()
                .map(|(a, v)| (a, v))
                .collect(),
            effects: HashMap::new(),
        })
    }
    #[test]
    fn test_effects() {
        let mut pol = Policy::default();
        pol.spec = datalog!( effect(amy, read). effect(amy, write) :- baz(A). ).compile().unwrap();
        let den = <Policy as justact::Policy>::truths(&pol);
        assert_eq!(den, Denotation {
            truths:  [make_effect("amy", "read")].into_iter().map(|a| (a, Some(true))).collect(),
            effects: [make_effect("amy", "read")]
                .into_iter()
                .map(|a| (a.clone(), Effect { fact: a, affector: Ident::new("amy".into(), None) }))
                .collect(),
        })
    }
}
