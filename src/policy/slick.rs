//  SLICK.rs
//    by Lut99
//
//  Created:
//    19 Dec 2024, 12:09:23
//  Last edited:
//    06 Jan 2025, 15:30:15
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements JustAct traits for the [`slick`]-crate.
//

use std::collections::HashMap;
use std::convert::Infallible;

use slick::text::Text;
use slick::{Atom, GroundAtom};
mod justact {
    pub use ::justact::auxillary::{Affectored, Identifiable};
    pub use ::justact::policies::{Denotation, Effect, Truth};
    pub use ::justact::sets::Set;
}
#[cfg(feature = "log")]
use log::trace;


/***** HELPERS *****/
/// It's either a Slick variable or constant.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AffectorAtom {
    Constant(Text),
    Variable(Text),
}





/***** LIBRARY *****/
/// Wraps a Slick (fact, truth) pair as a [`Truth`](justact::Truth).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Truth {
    /// The atom we're wrapping.
    pub fact:  GroundAtom,
    /// The value of the atom.
    ///
    /// Note that, Slick being Slick, it never occurs this is `Some(false)`. That can only happen
    /// implicitly by asking the truth value of an atom which is not in the denotation (and
    /// therefore false).
    pub value: Option<bool>,
}
impl justact::Identifiable for Truth {
    type Id = GroundAtom;

    #[inline]
    fn id(&self) -> &Self::Id { &self.fact }
}
impl justact::Truth for Truth {
    #[inline]
    fn value(&self) -> Option<bool> { self.value }
}

/// Wraps a Slick (truth, affector) pair as an [`Effect`](justact::Effect).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Effect {
    /// The truth wrapped.
    pub truth:    Truth,
    /// The affector who does this effect.
    pub affector: GroundAtom,
}
impl justact::Affectored for Effect {
    type AffectorId = GroundAtom;

    #[inline]
    fn affector_id(&self) -> &Self::AffectorId { &self.affector }
}
impl justact::Identifiable for Effect {
    type Id = <Truth as justact::Identifiable>::Id;

    #[inline]
    fn id(&self) -> &Self::Id { &self.truth.fact }
}
impl justact::Effect for Effect {}
impl justact::Truth for Effect {
    #[inline]
    fn value(&self) -> Option<bool> { self.truth.value }
}

/// Wraps a Slick denotation as a [`Denotation`](justact::Denotation).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Denotation {
    /// The set of truths computed from the slick denotation.
    truths:  HashMap<GroundAtom, Truth>,
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
        let mut truths: HashMap<GroundAtom, Truth> = HashMap::new();
        let mut effects: HashMap<GroundAtom, Effect> = HashMap::new();
        for (fact, value) in int.trues.into_iter().map(|v| (v, Some(true))).chain(int.unknowns.into_iter().map(|v| (v, None))) {
            // See if the fact matches the effect pattern
            fn match_effect(fact: &GroundAtom, value: Option<bool>, pat: &Atom) -> bool {
                #[cfg(feature = "log")]
                trace!("Finding effect pattern '{pat:?}' in '{fact:?}'");
                match (fact, pat) {
                    // If there's constants involved in the pattern, match that
                    (GroundAtom::Constant(l), Atom::Constant(r)) => {
                        trace!("--> fact '{l:?}' is a constant; pattern '{r:?}' is a constant");
                        l == r
                    },
                    (GroundAtom::Tuple(l), Atom::Tuple(r)) => {
                        trace!("--> fact '{l:?}' is a tuple; pattern '{r:?}' is a tuple");
                        if l.len() == r.len() {
                            // If the arity matches, then check if all the patterns match
                            for (l, r) in l.iter().zip(r.iter()) {
                                trace!("RECURSINGGGG");
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
                        trace!("--> fact '{fact:?}' is *something*; pattern '{var:?}' is a variable");
                        true
                    },
                    (fact, Atom::Wildcard) => {
                        trace!("--> fact '{fact:?}' is *something*; pattern '{pat:?}' is a wildcard");
                        true
                    },

                    // Otherwise, don't add
                    _ => {
                        trace!("--> fact '{fact:?}' is not a constant or tuple while the pattern is; and pattern '{pat:?}' is not a variable",);
                        false
                    },
                }
            }
            if match_effect(&fact, value, &pat) {
                // See if we have a constant affector or can match
                match affector {
                    AffectorAtom::Constant(c) => {
                        effects.insert(fact.clone(), Effect {
                            truth:    Truth { fact: fact.clone(), value: Some(true) },
                            affector: GroundAtom::Constant(c),
                        });
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
                                effects.insert(fact.clone(), Effect {
                                    truth:    Truth { fact: fact.clone(), value: Some(true) },
                                    affector: affector.clone(),
                                });
                            },
                            None => panic!("Did not find affector variable {v:?} in matched atom {fact:?}"),
                        }
                    },
                }
            }

            // Always add the truth as such
            truths.insert(fact.clone(), Truth { fact, value });
        }

        // OK, return the denotation!
        Self { truths, effects }
    }
}
impl justact::Set<Effect> for Denotation {
    type Error = Infallible;

    #[inline]
    fn get(&self, id: &<Truth as justact::Identifiable>::Id) -> Result<Option<&Effect>, Self::Error> { Ok(self.effects.get(id)) }

    #[inline]
    fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s Effect>, Self::Error>
    where
        Effect: 's + justact::Identifiable,
    {
        Ok(self.effects.values())
    }
}
impl justact::Set<Truth> for Denotation {
    type Error = Infallible;

    #[inline]
    fn get(&self, id: &<Truth as justact::Identifiable>::Id) -> Result<Option<&Truth>, Self::Error> { Ok(self.truths.get(id)) }

    #[inline]
    fn iter<'s>(&'s self) -> Result<impl Iterator<Item = &'s Truth>, Self::Error>
    where
        Truth: 's + justact::Identifiable,
    {
        Ok(self.truths.values())
    }
}
impl justact::Denotation for Denotation {
    type Effect = Effect;
    type Truth = Truth;
}





/***** TESTS *****/
#[cfg(test)]
mod tests {
    use humanlog::{DebugMode, HumanLogger};
    use slick::infer::Config;
    use slick::parse;

    use super::*;


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
                truth:    Truth { fact: make_flat_ground_atom_str("effect read by amy"), value: Some(true) },
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
                truth:    Truth { fact: make_flat_ground_atom_str("effect read by amy"), value: Some(true) },
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
                    truth:    Truth { fact: make_flat_ground_atom_str("effect read by amy"), value: Some(true) },
                    affector: make_flat_ground_atom_str("amy"),
                }),
                (make_flat_ground_atom_str("effect write by bob"), Effect {
                    truth:    Truth { fact: make_flat_ground_atom_str("effect write by bob"), value: Some(true) },
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
                    truth:    Truth { fact: make_flat_ground_atom_str("effect read by amy"), value: Some(true) },
                    affector: make_flat_ground_atom_str("amy"),
                }),
                (make_flat_ground_atom_str("effect write by bob"), Effect {
                    truth:    Truth { fact: make_flat_ground_atom_str("effect write by bob"), value: Some(true) },
                    affector: make_flat_ground_atom_str("bob"),
                })
            ])
        );
    }
}
