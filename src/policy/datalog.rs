//  DATALOG.rs
//    by Lut99
//
//  Created:
//    26 Nov 2024, 11:54:14
//  Last edited:
//    26 Nov 2024, 12:05:54
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements JustAct traits for the [`datalog`]-crate.
//

use datalog::ast::{Atom, Comma, Dot, Ident, Punctuated, Rule, Span, Spec};
use datalog::interpreter::interpretation::Interpretation;
use datalog::parser::parse;
use error_trace::trace;
use thiserror::Error;
mod justact {
    pub use ::justact::auxillary::{Authored, Identifiable};
    pub use ::justact::policy::{Extractor, Policy};
    pub use ::justact::set::LocalSet;
    pub use ::justact::statements::Message;
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
pub enum SyntaxError<'f, 's> {
    #[error("Failed to parse the input as valid UTF-8")]
    Utf8 {
        #[source]
        err: std::str::Utf8Error,
    },
    #[error("{}", trace!(("Failed to parse the input as valid Datalog"), err))]
    Datalog { err: datalog::parser::Error<'f, 's> },
}





/***** LIBRARY *****/
/// Wraps a [`Spec`] in order to implement [`Policy`](justact::Policy).
#[derive(Clone, Debug)]
pub struct Policy<'f, 's>(pub Spec<'f, 's>);
impl<'f, 's> justact::Policy for Policy<'f, 's> {
    type SemanticError = SemanticError<'f, 's>;

    fn assert_validity(&self) -> Result<(), Self::SemanticError> {
        // Simply derive and see if `error` occurs.
        let int: Interpretation<'f, 's> = match self.0.alternating_fixpoint() {
            Ok(int) => int,
            Err(err) => panic!("Failed to run derivation: {err}"),
        };
        let error_truth: Option<bool> = int.closed_world_truth(&Atom {
            ident: Ident { value: Span::new("<justact_prototype::policy::datalog::Policy::assert_validity()>", "error") },
            args:  None,
        });
        if error_truth == Some(false) { Ok(()) } else { Err(SemanticError::ErrorHolds { int }) }
    }
}



/// Represents the [`Extractor`] for Datalog's [`Spec`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Extractor;
impl<M> justact::Extractor<M> for Extractor
where
    M: justact::Authored<AuthorId = str> + justact::Identifiable<Id = str>,
{
    type Policy<'v> = Policy<'v, 'v> where Self: 'v;
    type SyntaxError<'v> = SyntaxError<'v, 'v> where Self: 'v;

    #[inline]
    fn extract<'v, R>(set: &justact::LocalSet<M, R>) -> Result<Self::Policy<'v>, Self::SyntaxError<'v>>
    where
        Self: Sized,
        M: justact::Authored + justact::Identifiable + justact::Message<'v>,
    {
        // Parse the policy in the messages one-by-one
        let mut add_error: bool = false;
        let mut spec = Spec { rules: vec![] };
        for msg in set {
            // Parse as UTF-8
            let snippet: &str = match std::str::from_utf8(msg.payload()) {
                Ok(snippet) => snippet,
                Err(err) => return Err(SyntaxError::Utf8 { err }),
            };

            // Parse as Datalog
            let msg_spec: Spec = match parse(msg.id_v(), snippet) {
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
                                if arg.ident().value.value() == msg.author() {
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
            let mut consequences: Punctuated<Atom, Comma> = Punctuated::new();
            consequences.push_first(Atom { ident: Ident { value: Span::new("<datalog::justact::Spec::extract_from>", "error") }, args: None });

            // Then add the rule
            spec.rules.push(Rule { consequences, tail: None, dot: Dot { span: Span::new("<datalog::justact::Spec::extract_from>", ".") } })
        }

        // OK, return the spec
        Ok(Policy(spec))
    }
}
