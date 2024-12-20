//  SLICK.rs
//    by Lut99
//
//  Created:
//    19 Dec 2024, 12:09:23
//  Last edited:
//    20 Dec 2024, 17:20:57
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


/***** HELPERS *****/
/// It's either a Slick variable or constant.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AffectorAtom {
    Constant(Text),
    Variable(Text),
}





/***** LIBRARY *****/
/// Wraps a Slick (fact, truth) pair as a [`Truth`](justact::Truth).
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
            fn collect_effect(fact: &GroundAtom, value: Option<bool>, pat: &Atom, affector: &AffectorAtom) -> Option<Effect> {
                match (fact, pat) {
                    // If there's constants involved in the pattern, match that
                    (GroundAtom::Constant(l), Atom::Constant(r)) => {
                        if l == r {
                            let affector: GroundAtom = match affector {
                                AffectorAtom::Constant(c) => GroundAtom::Constant(*c),
                                AffectorAtom::Variable(v) => panic!("Did not find affector variable {v:?} in matched atom {l:?}"),
                            };
                            Some(Effect { truth: Truth { fact: fact.clone(), value }, affector })
                        } else {
                            None
                        }
                    },
                    (GroundAtom::Tuple(l), Atom::Tuple(r)) => {
                        if l.len() == r.len() {
                            // If the arity matches, then check if all the patterns match
                            for (l, r) in l.iter().zip(r.iter()) {
                                if collect_effect(l, value, r, affector).is_none() {
                                    return None;
                                }
                            }

                            // It does! Now try to find the affector
                            let affector: GroundAtom = match affector {
                                AffectorAtom::Constant(c) => GroundAtom::Constant(*c),
                                AffectorAtom::Variable(v) => {
                                    fn get_var_contents(fact: &[GroundAtom], pat: &[Atom], var: Text) -> Option<GroundAtom> {
                                        for (i, arg) in pat.into_iter().enumerate() {
                                            match arg {
                                                Atom::Constant(_) => continue,
                                                Atom::Variable(v) => {
                                                    if *v == var {
                                                        return Some(fact[i].clone());
                                                    }
                                                },
                                                Atom::Wildcard => continue,
                                                Atom::Tuple(pat) => {
                                                    if let GroundAtom::Tuple(fact) = &fact[i] {
                                                        if fact.len() == pat.len() {
                                                            if let Some(res) = get_var_contents(fact, pat, var) {
                                                                return Some(res);
                                                            }
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                        None
                                    }
                                    match get_var_contents(l, r, *v) {
                                        Some(val) => val,
                                        None => panic!("Did not find affector variable {v:?} in matched atom {l:?}"),
                                    }
                                },
                            };

                            // Return the effect
                            Some(Effect { truth: Truth { fact: fact.clone(), value }, affector })
                        } else {
                            None
                        }
                    },

                    // If the pattern IS a variable, ez
                    (fact, Atom::Variable(var)) => {
                        let affector: GroundAtom = match affector {
                            AffectorAtom::Constant(c) => GroundAtom::Constant(*c),
                            AffectorAtom::Variable(v) => {
                                if var == v {
                                    fact.clone()
                                } else {
                                    panic!("Did not find affector variable {v:?} in matched atom {fact:?}")
                                }
                            },
                        };
                        Some(Effect { truth: Truth { fact: fact.clone(), value }, affector })
                    },
                    (fact, Atom::Wildcard) => {
                        let affector: GroundAtom = match affector {
                            AffectorAtom::Constant(c) => GroundAtom::Constant(*c),
                            AffectorAtom::Variable(v) => panic!("Did not find affector variable {v:?} in matched atom \"_\""),
                        };
                        Some(Effect { truth: Truth { fact: fact.clone(), value }, affector })
                    },

                    // Otherwise, don't add
                    _ => None,
                }
            }
            if let Some(effect) = collect_effect(&fact, value, &pat, &affector) {
                effects.insert(effect.truth.fact.clone(), effect);
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
