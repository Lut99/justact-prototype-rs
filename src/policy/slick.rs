//  SLICK.rs
//    by Lut99
//
//  Created:
//    19 Dec 2024, 12:09:23
//  Last edited:
//    15 Jan 2025, 14:54:31
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements JustAct traits for the [`slick`]-crate.
//

use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::ops::{Deref, DerefMut};

use slick::infer::Config;
use slick::text::Text;
use slick::{Atom, GroundAtom, Program, Rule, RuleBody, parse};
mod justact {
    pub use ::justact::auxillary::{Affectored, Identifiable};
    pub use ::justact::collections::map::Map;
    pub use ::justact::collections::set::{InfallibleSet, Set};
    pub use ::justact::messages::Message;
    pub use ::justact::policies::{Denotation, Effect, Extractor, Policy};
}
use error_trace::trace;
use thiserror::Error;


/***** ERRORS *****/
/// Defines errors that may occur when [extracting](Extractor::extract()) policy.
#[derive(Debug, Error)]
pub enum SyntaxError<'m> {
    #[error("{}", trace!(("Failed to iterate over messages in {what}"), &**err))]
    Iter { what: &'static str, err: Box<dyn 'm + Error> },
    #[error("{}", trace!(("Failed to parse the input as valid Slick"), err))]
    Slick { err: nom::Err<nom::error::VerboseError<&'m str>> },
}





/***** HELPERS *****/
/// It's either a Slick variable or constant.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AffectorAtom {
    Constant(Text),
    Variable(Text),
}





/***** LIBRARY *****/
/// Wraps a Slick (truth, affector) pair as an [`Effect`](justact::Effect).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Effect {
    /// The truth wrapped.
    pub fact:     GroundAtom,
    /// The affector who does this effect.
    pub affector: GroundAtom,
}
impl justact::Affectored for Effect {
    type AffectorId = GroundAtom;

    #[inline]
    fn affector_id(&self) -> &Self::AffectorId { &self.affector }
}
impl justact::Identifiable for Effect {
    type Id = GroundAtom;

    #[inline]
    fn id(&self) -> &Self::Id { &self.fact }
}
impl justact::Effect for Effect {
    type Fact = GroundAtom;

    #[inline]
    fn fact(&self) -> &Self::Fact { &self.fact }
}

/// Wraps a Slick denotation as a [`Denotation`](justact::Denotation).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Denotation {
    /// The set of truths computed from the slick denotation.
    truths:  HashMap<GroundAtom, Option<bool>>,
    /// The set of effects computed from the slick denotation.
    effects: HashMap<GroundAtom, Effect>,
}
impl Default for Denotation {
    #[inline]
    fn default() -> Self { Self { truths: HashMap::new(), effects: HashMap::new() } }
}
impl Denotation {
    /// Creates a new Denotation from a Datalog [`Interpretation`].
    ///
    /// Note that, Slick being Slick, the Denotation only carries truths and unknowns. False facts
    /// are defined implicitly by [asking their truth](Denotation::truth_of()) and finding it is
    /// not in the Denotation.
    ///
    /// # Arguments
    /// - `int`: The [`Denotation`](slick::infer::Denotation) to build this Denotation from.
    /// - `pat`: An [`Atom`] that describes a pattern for recognizing effets.
    /// - `affector`: An [`AffectorAtom`] that describes how to extract the affector from the effect.
    ///
    /// # Returns
    /// A new Denotation that is JustAct^{TM} compliant.
    #[inline]
    pub fn from_interpretation(int: slick::infer::Denotation, pat: Atom, affector: AffectorAtom) -> Self {
        let mut truths: HashMap<GroundAtom, Option<bool>> = HashMap::new();
        let mut effects: HashMap<GroundAtom, Effect> = HashMap::new();
        for (fact, value) in int.trues.into_iter().map(|v| (v, Some(true))).chain(int.unknowns.into_iter().map(|v| (v, None))) {
            // See if the fact matches the effect pattern
            fn match_effect(fact: &GroundAtom, value: Option<bool>, pat: &Atom) -> bool {
                #[cfg(feature = "log")]
                log::trace!("Finding effect pattern '{pat:?}' in '{fact:?}'");
                match (fact, pat) {
                    // If there's constants involved in the pattern, match that
                    (GroundAtom::Constant(l), Atom::Constant(r)) => {
                        log::trace!("--> fact '{l:?}' is a constant; pattern '{r:?}' is a constant");
                        l == r
                    },
                    (GroundAtom::Tuple(l), Atom::Tuple(r)) => {
                        log::trace!("--> fact '{l:?}' is a tuple; pattern '{r:?}' is a tuple");
                        if l.len() == r.len() {
                            // If the arity matches, then check if all the patterns match
                            for (l, r) in l.iter().zip(r.iter()) {
                                log::trace!("RECURSINGGGG");
                                if !match_effect(l, value, r) {
                                    return false;
                                }
                            }
                            true
                        } else {
                            false
                        }
                    },

                    // If the pattern IS a variable, ez
                    (fact, Atom::Variable(var)) => {
                        log::trace!("--> fact '{fact:?}' is *something*; pattern '{var:?}' is a variable");
                        true
                    },
                    (fact, Atom::Wildcard) => {
                        log::trace!("--> fact '{fact:?}' is *something*; pattern '{pat:?}' is a wildcard");
                        true
                    },

                    // Otherwise, don't add
                    _ => {
                        log::trace!("--> fact '{fact:?}' is not a constant or tuple while the pattern is; and pattern '{pat:?}' is not a variable",);
                        false
                    },
                }
            }
            if match_effect(&fact, value, &pat) {
                // See if we have a constant affector or can match
                match affector {
                    AffectorAtom::Constant(c) => {
                        effects.insert(fact.clone(), Effect { fact: fact.clone(), affector: GroundAtom::Constant(c) });
                    },
                    AffectorAtom::Variable(v) => {
                        fn get_var_contents<'f>(fact: &'f GroundAtom, pat: &Atom, affector_var: &Text) -> Option<&'f GroundAtom> {
                            match pat {
                                Atom::Constant(_) => None,
                                Atom::Tuple(pat) => {
                                    for (fact, pat) in if let GroundAtom::Tuple(fact) = fact { fact.iter() } else { unreachable!() }.zip(pat.iter()) {
                                        if let Some(res) = get_var_contents(fact, pat, affector_var) {
                                            return Some(res);
                                        } else {
                                            continue;
                                        }
                                    }
                                    None
                                },
                                Atom::Variable(pat) => {
                                    if pat == affector_var {
                                        Some(fact)
                                    } else {
                                        None
                                    }
                                },
                                Atom::Wildcard => Some(fact),
                            }
                        }
                        match get_var_contents(&fact, &pat, &v) {
                            Some(affector) => {
                                effects.insert(fact.clone(), Effect { fact: fact.clone(), affector: affector.clone() });
                            },
                            None => panic!("Did not find affector variable {v:?} in matched atom {fact:?}"),
                        }
                    },
                }
            }

            // Always add the truth as such
            truths.insert(fact, value);
        }

        // OK, return the denotation!
        Self { truths, effects }
    }



    /// Checks if this interpretation contains a fact that would make the policy invalid.
    ///
    /// # Returns
    /// True if the parent policy is valid, false otherwise.
    pub fn is_valid(&self) -> bool {
        // Check whether error is true in the truths
        let atom = GroundAtom::Constant(Text::from_str("error"));
        for fact in <Denotation as justact::InfallibleSet<GroundAtom>>::iter(self) {
            match fact {
                GroundAtom::Constant(c) => {
                    if c == &Text::from_str("error") {
                        return false;
                    }
                },
                GroundAtom::Tuple(t) => {
                    if t.len() >= 1 && t[0] == atom {
                        return false;
                    }
                },
            }
        }
        true
    }
}
impl justact::Map<Effect> for Denotation {
    type Error = Infallible;

    #[inline]
    fn get(&self, id: &<Effect as justact::Identifiable>::Id) -> Result<Option<&Effect>, Self::Error> { Ok(self.effects.get(id)) }

    #[inline]
    fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s Effect>, Self::Error>
    where
        Effect: 's + justact::Identifiable,
    {
        Ok(self.effects.values())
    }
}
impl justact::Set<GroundAtom> for Denotation {
    type Error = Infallible;

    #[inline]
    fn get(&self, elem: &GroundAtom) -> Result<Option<&GroundAtom>, Self::Error> { Ok(self.truths.get_key_value(elem).map(|(k, _)| k)) }

    #[inline]
    fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s GroundAtom>, Self::Error>
    where
        GroundAtom: 's,
    {
        Ok(self.truths.keys())
    }
}
impl justact::Denotation for Denotation {
    type Effect = Effect;
    type Fact = GroundAtom;

    #[inline]
    fn truth_of(&self, fact: &Self::Fact) -> Option<bool> { self.truths.get(fact).cloned().unwrap_or(Some(false)) }
}



/// Wraps a [`Program`] in order to implement [`Policy`](justact::Policy).
#[derive(Clone, Debug)]
pub struct Policy {
    /// The pattern to match effects.
    pat:      Atom,
    /// How to find the affector from effects.
    affector: AffectorAtom,
    /// The program we wrap with actual policy.
    program:  Program,
}
impl Default for Policy {
    #[inline]
    fn default() -> Self {
        Self {
            pat:      Atom::Tuple(vec![
                Atom::Constant(Text::from_str("effect")),
                Atom::Variable(Text::from_str("Effect")),
                Atom::Constant(Text::from_str("by")),
                Atom::Variable(Text::from_str("Affector")),
            ]),
            affector: AffectorAtom::Variable(Text::from_str("Affector")),
            program:  Program { rules: Vec::new() },
        }
    }
}
impl Policy {
    /// Updates the pattern that matches Slick atoms to match effects.
    ///
    /// By default, any Slick atom of the shape 'effect Affector by Effect` is seen as an effect.
    /// But this may not always be desired; and as such, another pattern can be given.
    ///
    /// The pattern can use variables as wildcards. Then, the `affector` can optionally use one of
    /// those to communicate one of those variables encodes the affector.
    ///
    /// # Arguments
    /// - `pat`: The pattern used to match effects.
    /// - `affector`: What affector to provide for effects (given as a special [`AffectorAtom`]).
    #[inline]
    pub fn update_effect_pattern(&mut self, pat: Atom, affector: AffectorAtom) {
        self.pat = pat;
        self.affector = affector;
    }

    /// Returns the program.
    ///
    /// # Returns
    /// A reference to the internal [`Program`].
    #[inline]
    pub fn program(&self) -> &Program { &self.program }
    /// Returns the program mutably.
    ///
    /// # Returns
    /// A mutable reference to the internal [`Program`].
    #[inline]
    pub fn program_mut(&mut self) -> &mut Program { &mut self.program }
    /// Returns the program by consuming this Policy.
    ///
    /// # Returns
    /// The internal [`Program`].
    #[inline]
    pub fn into_program(self) -> Program { self.program }
}
impl justact::Policy for Policy {
    type Denotation = Denotation;


    #[inline]
    fn is_valid(&self) -> bool { self.truths().is_valid() }

    #[inline]
    fn truths(&self) -> Self::Denotation {
        let atom = GroundAtom::Tuple(vec![
            GroundAtom::Constant(Text::from_str("error")),
            GroundAtom::Tuple(vec![GroundAtom::Constant(Text::from_str("inference")), GroundAtom::Constant(Text::from_str("failure"))]),
        ]);
        match self.program.clone().denotation(&Config::default()) {
            Ok(den) => Denotation::from_interpretation(den, self.pat.clone(), self.affector.clone()),
            Err(err) => {
                #[cfg(feature = "log")]
                log::error!("Failed to compute denotation: {:?}\n\nProgram:\n{}\n{:?}\n{}\n", err, "-".repeat(80), self.program, "-".repeat(80));
                Denotation { truths: HashMap::from([(atom, Some(true))]), effects: HashMap::new() }
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
    fn compose_mut(&mut self, other: Self) { self.program.rules.extend(other.program.rules); }
}
impl Deref for Policy {
    type Target = Program;
    #[inline]
    fn deref(&self) -> &Self::Target { &self.program }
}
impl DerefMut for Policy {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.program }
}



/// Represents the [`Extractor`] for Slick's [`Program`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Extractor;
impl justact::Extractor<str, str, str> for Extractor {
    type Policy<'m> = Policy;
    type Error<'m> = SyntaxError<'m>;


    #[inline]
    fn extract<'m, M: 'm + justact::Message<Id = str, AuthorId = str, Payload = str>>(
        &self,
        msgs: &'m impl justact::Map<M>,
    ) -> Result<Self::Policy<'m>, Self::Error<'m>> {
        // Attempt to iterate over the messages
        let iter = msgs.iter().map_err(|err| SyntaxError::Iter { what: std::any::type_name::<M>(), err: Box::new(err) })?;

        // Parse the policy in the messages one-by-one
        let mut policy = Policy::default();
        for msg in iter {
            // Parse as UTF-8
            let snippet: &str = msg.payload();

            // Parse as Slick
            let mut msg_prog: Program = match parse::program(snippet) {
                Ok((_, prog)) => prog,
                Err(err) => return Err(SyntaxError::Slick { err }),
            };

            // // Search for any illegal `Fact within (Author M)` messages
            // 'rule: for rule in &mut msg_prog.rules {
            //     for cons in &rule.consequents {
            //         if let Atom::Tuple(atoms) = cons {
            //             if let [Atom::Variable(_), Atom::Constant(within), Atom::Tuple(nested)] = atoms.as_slice() {
            //                 if let [Atom::Variable(_), Atom::Variable(_)] = nested.as_slice() {
            //                     if within == &Text::from_str("within") {
            //                         // Alright, full match of the illegal rule; replace it with error as a whole
            //                         *rule = Rule {
            //                             consequents: vec![
            //                                 Atom::Constant(Text::from_str("error")),
            //                                 Atom::Tuple(vec![
            //                                     Atom::Constant(Text::from_str("illegal")),
            //                                     Atom::Constant(Text::from_str("fact")),
            //                                     Atom::Constant(Text::from_str("within")),
            //                                 ]),
            //                             ],
            //                             rule_body:   RuleBody { pos_antecedents: Vec::new(), neg_antecedents: Vec::new(), checks: Vec::new() },
            //                         };
            //                         continue 'rule;
            //                     }
            //                 }
            //             }
            //         }
            //     }
            // }

            // Generate additional `within`-facts
            let mut within: Vec<Rule> = Vec::with_capacity(msg_prog.rules.len());
            for rule in &msg_prog.rules {
                for cons in &rule.consequents {
                    within.push(Rule {
                        consequents: vec![Atom::Tuple(vec![
                            cons.clone(),
                            Atom::Constant(Text::from_str("within")),
                            Atom::Tuple(vec![Atom::Constant(Text::from_str(msg.author_id())), Atom::Constant(Text::from_str(msg.id()))]),
                        ])],
                        rule_body:   RuleBody { pos_antecedents: vec![cons.clone()], neg_antecedents: Vec::new(), checks: Vec::new() },
                    });
                }
            }
            msg_prog.rules.extend(within);

            // OK, now we can add all the rules together
            policy.program.rules.extend(msg_prog.rules);
        }

        // OK, return the spec
        Ok(policy)
    }
}





/***** TESTS *****/
#[cfg(test)]
mod tests {
    use humanlog::{DebugMode, HumanLogger};
    use slick::infer::Config;
    use slick::{Rule, RuleBody};

    use super::*;
    mod justact {
        pub use ::justact::auxillary::Authored;
        pub use ::justact::messages::MessageSet;

        pub use super::super::justact::*;
    }


    /// Generates the effect pattern.
    #[inline]
    fn make_pattern() -> Atom {
        Atom::Tuple(vec![
            Atom::Constant(Text::from_str("effect")),
            Atom::Variable(Text::from_str("Effect")),
            Atom::Constant(Text::from_str("by")),
            Atom::Variable(Text::from_str("Affector")),
        ])
    }

    /// Generates a ground atom.
    #[inline]
    #[track_caller]
    fn make_flat_ground_atom_str(s: &str) -> GroundAtom { parse::ground_atom(s).unwrap().1 }

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
        fn new(id: <Self::Id as ToOwned>::Owned, author_id: <Self::AuthorId as ToOwned>::Owned, payload: <Self::Payload as ToOwned>::Owned) -> Self
        where
            Self: Sized,
        {
            Self { id: id.to_owned(), author_id: author_id.to_owned(), payload: payload.to_owned() }
        }

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
    }


    #[test]
    fn test_extract_policy_single() {
        let msg = Message { id: "a".into(), author_id: "amy".into(), payload: "foo. bar if baz A.".into() };
        let pol = <Extractor as justact::Extractor<str, str, str>>::extract(&Extractor, &msg).unwrap();
        assert_eq!(pol.program, Program {
            rules: vec![
                Rule {
                    consequents: vec![Atom::Constant(Text::from_str("foo"))],
                    rule_body:   RuleBody { pos_antecedents: vec![], neg_antecedents: vec![], checks: vec![] },
                },
                Rule {
                    consequents: vec![Atom::Constant(Text::from_str("bar"))],
                    rule_body:   RuleBody {
                        pos_antecedents: vec![Atom::Tuple(vec![Atom::Constant(Text::from_str("baz")), Atom::Variable(Text::from_str("A"))])],
                        neg_antecedents: vec![],
                        checks: vec![],
                    },
                },
                Rule {
                    consequents: vec![Atom::Tuple(vec![
                        Atom::Constant(Text::from_str("foo")),
                        Atom::Constant(Text::from_str("within")),
                        Atom::Tuple(vec![Atom::Constant(Text::from_str("amy")), Atom::Constant(Text::from_str("a"))])
                    ])],
                    rule_body:   RuleBody { pos_antecedents: vec![Atom::Constant(Text::from_str("foo"))], neg_antecedents: vec![], checks: vec![] },
                },
                Rule {
                    consequents: vec![Atom::Tuple(vec![
                        Atom::Constant(Text::from_str("bar")),
                        Atom::Constant(Text::from_str("within")),
                        Atom::Tuple(vec![Atom::Constant(Text::from_str("amy")), Atom::Constant(Text::from_str("a"))])
                    ])],
                    rule_body:   RuleBody { pos_antecedents: vec![Atom::Constant(Text::from_str("bar"))], neg_antecedents: vec![], checks: vec![] },
                }
            ],
        });
    }
    #[test]
    fn test_extract_policy_multi() {
        // Construct a set of messages
        let msg1 = Message { id: "a".into(), author_id: "amy".into(), payload: "foo.".into() };
        let msg2 = Message { id: "b".into(), author_id: "bob".into(), payload: "bar :- baz(A).".into() };
        let msgs = justact::MessageSet::from([msg1, msg2]);

        // Extract the policy from it
        let pol = <Extractor as justact::Extractor<str, str, str>>::extract(&Extractor, &msgs).unwrap();
        // NOTE: MessageSet collects messages unordered, so the rules may be in any order
        assert!(
            pol.program
                == Program {
                    rules: vec![
                        Rule {
                            consequents: vec![Atom::Constant(Text::from_str("foo"))],
                            rule_body:   RuleBody { pos_antecedents: vec![], neg_antecedents: vec![], checks: vec![] },
                        },
                        Rule {
                            consequents: vec![Atom::Tuple(vec![
                                Atom::Constant(Text::from_str("foo")),
                                Atom::Constant(Text::from_str("within")),
                                Atom::Tuple(vec![Atom::Constant(Text::from_str("amy")), Atom::Constant(Text::from_str("a"))])
                            ])],
                            rule_body:   RuleBody {
                                pos_antecedents: vec![Atom::Constant(Text::from_str("foo"))],
                                neg_antecedents: vec![],
                                checks: vec![],
                            },
                        },
                        Rule {
                            consequents: vec![Atom::Constant(Text::from_str("bar"))],
                            rule_body:   RuleBody {
                                pos_antecedents: vec![Atom::Tuple(vec![Atom::Constant(Text::from_str("baz")), Atom::Variable(Text::from_str("A"))])],
                                neg_antecedents: vec![],
                                checks: vec![],
                            },
                        },
                        Rule {
                            consequents: vec![Atom::Tuple(vec![
                                Atom::Constant(Text::from_str("bar")),
                                Atom::Constant(Text::from_str("within")),
                                Atom::Tuple(vec![Atom::Constant(Text::from_str("bob")), Atom::Constant(Text::from_str("b"))])
                            ])],
                            rule_body:   RuleBody {
                                pos_antecedents: vec![Atom::Constant(Text::from_str("bar"))],
                                neg_antecedents: vec![],
                                checks: vec![],
                            },
                        },
                    ],
                }
                || pol.program
                    == Program {
                        rules: vec![
                            Rule {
                                consequents: vec![Atom::Constant(Text::from_str("bar"))],
                                rule_body:   RuleBody {
                                    pos_antecedents: vec![Atom::Tuple(vec![
                                        Atom::Constant(Text::from_str("baz")),
                                        Atom::Variable(Text::from_str("A"))
                                    ])],
                                    neg_antecedents: vec![],
                                    checks: vec![],
                                },
                            },
                            Rule {
                                consequents: vec![Atom::Tuple(vec![
                                    Atom::Constant(Text::from_str("bar")),
                                    Atom::Constant(Text::from_str("within")),
                                    Atom::Tuple(vec![Atom::Constant(Text::from_str("bob")), Atom::Constant(Text::from_str("b"))])
                                ])],
                                rule_body:   RuleBody {
                                    pos_antecedents: vec![Atom::Constant(Text::from_str("bar"))],
                                    neg_antecedents: vec![],
                                    checks: vec![],
                                },
                            },
                            Rule {
                                consequents: vec![Atom::Constant(Text::from_str("foo"))],
                                rule_body:   RuleBody { pos_antecedents: vec![], neg_antecedents: vec![], checks: vec![] },
                            },
                            Rule {
                                consequents: vec![Atom::Tuple(vec![
                                    Atom::Constant(Text::from_str("foo")),
                                    Atom::Constant(Text::from_str("within")),
                                    Atom::Tuple(vec![Atom::Constant(Text::from_str("amy")), Atom::Constant(Text::from_str("a"))])
                                ])],
                                rule_body:   RuleBody {
                                    pos_antecedents: vec![Atom::Constant(Text::from_str("foo"))],
                                    neg_antecedents: vec![],
                                    checks: vec![],
                                },
                            },
                        ],
                    }
        );
    }

    #[test]
    fn test_is_valid() {
        let mut pol = Policy::default();
        pol.program = parse::program("foo. bar if baz A.").unwrap().1;
        assert!(<Policy as justact::Policy>::is_valid(&pol));
    }
    #[test]
    fn test_is_not_valid() {
        let mut pol = Policy::default();
        pol.program = parse::program("error if foo. foo.").unwrap().1;
        assert!(!<Policy as justact::Policy>::is_valid(&pol));
    }

    #[test]
    fn test_truths() {
        let mut pol = Policy::default();
        pol.program = parse::program("foo. bar if baz A.").unwrap().1;
        let den = <Policy as justact::Policy>::truths(&pol);
        assert_eq!(den, Denotation {
            truths:  [GroundAtom::Constant(Text::from_str("foo"))].into_iter().map(|a| (a, Some(true))).collect(),
            effects: HashMap::new(),
        })
    }
    #[test]
    fn test_effects() {
        let mut pol = Policy::default();
        pol.program = parse::program("effect read by amy. effect write by amy if baz A.").unwrap().1;
        let den = <Policy as justact::Policy>::truths(&pol);
        assert_eq!(den, Denotation {
            truths:  [make_flat_ground_atom_str("effect read by amy")].into_iter().map(|a| (a, Some(true))).collect(),
            effects: [make_flat_ground_atom_str("effect read by amy")]
                .into_iter()
                .map(|a| (a.clone(), Effect { fact: a, affector: GroundAtom::Constant(Text::from_str("amy")) }))
                .collect(),
        })
    }

    /// Tests whether the extraction of effects works as expected when there's nothing to extract.
    #[test]
    fn test_denotation_effects_none() {
        #[cfg(feature = "log")]
        if std::env::var("LOGGER").ok() == Some("1".into()) {
            if let Err(err) = HumanLogger::terminal(DebugMode::Full).init() {
                eprintln!("WARNING: Failed to setup logger: {err}");
            }
        }

        // Empty pattern, effect program
        let program = parse::program("effect read by amy.").unwrap();
        let int = program.1.denotation(&Config::default()).unwrap();
        let den = Denotation::from_interpretation(int, Atom::Tuple(vec![]), AffectorAtom::Constant(Text::from_str("affector")));
        assert!(den.effects.is_empty());

        // Non-empty pattern, empty program
        let program = parse::program("").unwrap();
        let int = program.1.denotation(&Config::default()).unwrap();
        let den = Denotation::from_interpretation(int, make_pattern(), AffectorAtom::Variable(Text::from_str("Affector")));
        assert!(den.effects.is_empty());

        // Non-empty pattern, non-empty & non-effect program
        let program = parse::program("amy. amy reads an effect. effect write of amy.").unwrap();
        let int = program.1.denotation(&Config::default()).unwrap();
        let den = Denotation::from_interpretation(int, make_pattern(), AffectorAtom::Variable(Text::from_str("Affector")));
        assert!(den.effects.is_empty());
    }
    /// Tests whether the extraction of effects works as expected when there are effects to extract.
    #[test]
    fn test_denotation_effects_some() {
        #[cfg(feature = "log")]
        if std::env::var("LOGGER").ok() == Some("1".into()) {
            if let Err(err) = HumanLogger::terminal(DebugMode::Full).init() {
                eprintln!("WARNING: Failed to setup logger: {err}");
            }
        }

        // Match a single effect
        let program = parse::program("effect read by amy.").unwrap();
        let int = program.1.denotation(&Config::default()).unwrap();
        let den = Denotation::from_interpretation(int, make_pattern(), AffectorAtom::Variable(Text::from_str("Affector")));
        assert_eq!(
            den.effects,
            HashMap::from([(make_flat_ground_atom_str("effect read by amy"), Effect {
                fact:     make_flat_ground_atom_str("effect read by amy"),
                affector: make_flat_ground_atom_str("amy"),
            })])
        );

        // Match a single effect, but with noise
        let program = parse::program("effect write b bob. effect read by amy. foo bar baz.").unwrap();
        let int = program.1.denotation(&Config::default()).unwrap();
        let den = Denotation::from_interpretation(int, make_pattern(), AffectorAtom::Variable(Text::from_str("Affector")));
        assert_eq!(
            den.effects,
            HashMap::from([(make_flat_ground_atom_str("effect read by amy"), Effect {
                fact:     make_flat_ground_atom_str("effect read by amy"),
                affector: make_flat_ground_atom_str("amy"),
            })])
        );

        // Match a multiple effects
        let program = parse::program("effect read by amy. effect write by bob.").unwrap();
        let int = program.1.denotation(&Config::default()).unwrap();
        let den = Denotation::from_interpretation(int, make_pattern(), AffectorAtom::Variable(Text::from_str("Affector")));
        assert_eq!(
            den.effects,
            HashMap::from([
                (make_flat_ground_atom_str("effect read by amy"), Effect {
                    fact:     make_flat_ground_atom_str("effect read by amy"),
                    affector: make_flat_ground_atom_str("amy"),
                }),
                (make_flat_ground_atom_str("effect write by bob"), Effect {
                    fact:     make_flat_ground_atom_str("effect write by bob"),
                    affector: make_flat_ground_atom_str("bob"),
                })
            ])
        );

        // Match a multiple effects, but with noise
        let program = parse::program("effect read by amy. effect read by. effect write by bob. foo. effect.").unwrap();
        let int = program.1.denotation(&Config::default()).unwrap();
        let den = Denotation::from_interpretation(int, make_pattern(), AffectorAtom::Variable(Text::from_str("Affector")));
        assert_eq!(
            den.effects,
            HashMap::from([
                (make_flat_ground_atom_str("effect read by amy"), Effect {
                    fact:     make_flat_ground_atom_str("effect read by amy"),
                    affector: make_flat_ground_atom_str("amy"),
                }),
                (make_flat_ground_atom_str("effect write by bob"), Effect {
                    fact:     make_flat_ground_atom_str("effect write by bob"),
                    affector: make_flat_ground_atom_str("bob"),
                })
            ])
        );
    }

    /// Tests if the author rules work as expected.
    #[test]
    fn test_reflection() {
        // First, see if the derivation works.
        let msg1 = Message { id: "a".into(), author_id: "amy".into(), payload: "foo. (bar foo) if foo. baz X if bar X.".into() };
        let msg2 = Message { id: "b".into(), author_id: "bob".into(), payload: "qux X if baz X.".into() };
        let pol = <Extractor as justact::Extractor<str, str, str>>::extract(&Extractor, &justact::MessageSet::from([msg1, msg2])).unwrap();
        let den = <Policy as justact::Policy>::truths(&pol);
        assert_eq!(den, Denotation {
            truths:  [
                make_flat_ground_atom_str("foo"),
                make_flat_ground_atom_str("bar foo"),
                make_flat_ground_atom_str("baz foo"),
                make_flat_ground_atom_str("qux foo"),
                make_flat_ground_atom_str("foo within (amy a)"),
                make_flat_ground_atom_str("(bar foo) within (amy a)"),
                make_flat_ground_atom_str("(baz foo) within (amy a)"),
                make_flat_ground_atom_str("(qux foo) within (bob b)"),
            ]
            .into_iter()
            .map(|a| (a.clone(), Some(true)))
            .collect(),
            effects: HashMap::new(),
        });
    }
}
