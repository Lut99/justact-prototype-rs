//  INTERFACE.rs
//    by Lut99
//
//  Created:
//    16 Apr 2024, 10:58:56
//  Last edited:
//    26 Nov 2024, 11:51:26
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a collection of users to format messages nicesly.
//

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FResult};

use console::{style, Style};
use justact::agreements::Agreement;
use justact::auxillary::{Authored as _, Identifiable as _};
use justact::set::LocalSet;
use justact::statements::{Action, AuditExplanation, Message as _};
use justact::times::Timestamp;

use crate::statements::Message;


/***** FORMATTERS *****/
/// Formats a [`MessageSet`] with proper indentation and such.
pub struct MessageSetFormatter<'m, V, S, P, I> {
    /// The message to format.
    msgs:   &'m LocalSet<V, S>,
    /// Some prefix to use when writing the message.
    prefix: P,
    /// The indentation to use while formatting.
    indent: I,
}
impl<'m, V: Displayable, S, P: Display, I: Display> Display for MessageSetFormatter<'m, V, S, P, I> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        // Write the messages, one-by-one
        writeln!(f, "{} {{", self.prefix)?;
        for msg in self.msgs {
            write!(f, "{}    {}", self.indent, msg.display("Message", &format!("{}    ", self.indent)))?;
        }
        writeln!(f, "{}}}", self.indent)
    }
}

/// Formats an [`Agreement`] with proper indentation and such.
pub struct AgreementFormatter<'a, P, I, M> {
    /// The agreement to format.
    agr:    &'a Agreement<M>,
    /// Some prefix to use when writing the message.
    prefix: P,
    /// The indentation to use while formatting.
    indent: I,
}
impl<'a, P: Display, I: Display> Display for AgreementFormatter<'a, P, I, Message> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        // First, get the agreement's payload as UTF-8
        let spayload: Cow<str> = String::from_utf8_lossy((&self.agr.msg).payload());

        // Write the agreement
        writeln!(
            f,
            "{} '{}' by '{}' (applies at {}) {{",
            self.prefix,
            style(self.agr.id()).bold(),
            style(self.agr.msg.author()).bold(),
            style(self.agr.timestamp).bold(),
        )?;
        writeln!(f, "{}    {}", self.indent, spayload.replace('\n', &format!("\n{}    ", self.indent)).trim_end())?;
        writeln!(f, "{}}}", self.indent)
    }
}





/***** INTERFACES *****/
/// Gives us an opportunity to implement some external functions on JustAct implementations.
pub trait Displayable {
    /// The formatter returned to display.
    type Formatter<'s, P: Display, I: Display>: Display
    where
        Self: 's;


    /// Returns some formatter that formats this Displayable.
    ///
    /// # Arguments
    /// - `prefix`: Some prefix/name for this message (e.g., `Message` or `Justification`).
    /// - `indent`: Something to write as indentation after all newlines (i.e., on all lines except the first).
    ///
    /// # Returns
    /// A `Self::Formatter` that implements [`Display`].
    fn display<'s, P: Display, I: Display>(&'s self, prefix: P, indent: I) -> Self::Formatter<'s, P, I>;
}

// Impls for JustAct types
impl<V: Displayable, S> Displayable for LocalSet<V, S> {
    type Formatter<'s, P: Display, I: Display> = MessageSetFormatter<'s, V, S, P, I> where Self: 's;

    #[inline]
    fn display<'s, P: Display, I: Display>(&'s self, prefix: P, indent: I) -> Self::Formatter<'s, P, I> {
        MessageSetFormatter { msgs: self, prefix, indent }
    }
}
impl Displayable for Agreement<Message> {
    type Formatter<'s, P: Display, I: Display> = AgreementFormatter<'s, P, I, Message> where Self: 's;

    #[inline]
    fn display<'s, P: Display, I: Display>(&'s self, prefix: P, indent: I) -> Self::Formatter<'s, P, I> {
        AgreementFormatter { agr: self, prefix, indent }
    }
}

// Pointer-like impls
impl<'v, D: Displayable> Displayable for &'v D {
    type Formatter<'s, P: Display, I: Display> = D::Formatter<'s, P, I> where Self: 's;

    #[inline]
    fn display<'s, P: Display, I: Display>(&'s self, prefix: P, indent: I) -> Self::Formatter<'s, P, I> { D::display(self, prefix, indent) }
}





/***** LIBRARY *****/
/// Implements a [`justact::Interface`] that allows agents to communicate with the simulation environment's end user.
#[derive(Clone, Debug)]
pub struct Interface {
    /// The mapping of agents to their styles.
    styles: HashMap<String, Style>,
}

impl Interface {
    /// Constructor for the Interface.
    ///
    /// # Returns
    /// A new Interface ready for use in the simulation.
    #[inline]
    pub fn new() -> Self { Self { styles: HashMap::new() } }

    /// Registers the style for a new agent.
    ///
    /// Note that all other functions panic if you call it for an agent that hasn't been registered yet.
    ///
    /// # Arguments
    /// - `id`: The identifier of the agent to register.
    /// - `style`: The style to use when formatting the agent's `id`entifier.
    #[inline]
    pub fn register(&mut self, id: &str, style: Style) { self.styles.insert(id.into(), style); }



    /// Logs an arbitrary message to stdout.
    ///
    /// # Arguments
    /// - `id`: The identifier of the agent who is logging.
    /// - `msg`: Some message (retrieved as [`Display`]) to show.
    pub fn log(&self, id: &str, msg: impl Display) {
        println!("{}{}{} {}\n", style("[INFO] [").bold(), self.styles.get(id).unwrap().apply_to(id), style("]").bold(), msg);
    }

    /// Logs the statement of a [`Message`] to stdout.
    ///
    /// # Arguments
    /// - `id`: The identifier of the agent who is logging.
    /// - `msg`: Some [`Message`] to emit.
    pub fn log_state(&self, id: &str, msg: &Message) {
        // Write the main log-line
        println!("{}{}{} Emitted message '{}'", style("[INFO] [").bold(), self.styles.get(id).unwrap().apply_to(id), style("]").bold(), msg.id());

        // Write the message
        println!(" └> {}", msg.display("Message", "    "));
        println!();

        // Done
    }

    /// Logs the enactment of an [`Action`] over [`Message`]s to stdout.
    ///
    /// # Arguments
    /// - `id`: The identifier of the agent who is logging.
    /// - `act`: Some [`Action`] to emit.
    pub fn log_enact(&self, id: &str, act: &Action<Message>) {
        let just: LocalSet<&Message> = act.justification();

        // Retrieve the message IDs for the justication
        let mut just_ids: String = String::new();
        for msg in just.iter() {
            if !just_ids.is_empty() {
                just_ids.push_str(", ");
            }
            just_ids.push_str(&msg.id().to_string());
        }

        // Write the main log-line
        println!(
            "{}{}{} Enacted message '{}' using '{}' (basis '{}')",
            style("[INFO] [").bold(),
            self.styles.get(id).unwrap().apply_to(id),
            style("]").bold(),
            act.enacts().id(),
            just_ids,
            act.basis().id(),
        );

        // Write the sets
        print!(" ├> {}", act.basis().display("Basis", " |  "));
        print!(" ├> {}", just.display("Justification", " |  "));
        print!(" └> {}", act.enacts().display("Enacts", "    "));
        println!();

        // Done
    }



    /// Logs that an agent started synchronizing a new time.
    ///
    /// # Arguments
    /// - `id`: The identifier of the agent who is logging.
    /// - `time`: The [`Timestamp`] that will be advanced if synchronized.
    pub fn log_advance_start(&self, id: &str, time: Timestamp) {
        // Write the main log-line
        println!(
            "{}{}{} Initiated synchronization to advance time to {}",
            style("[INFO] [").bold(),
            self.styles.get(id).unwrap().apply_to(id),
            style("]").bold(),
            style(time).bold()
        );
        println!();
    }

    /// Logs that all agents agreed to synchronize time.
    ///
    /// # Arguments
    /// - `time`: The [`Timestamp`] that will be advanced if synchronized.
    pub fn log_advance(&self, time: Timestamp) {
        // Write the main log-line
        println!(
            "{}{}{} Time advanced to {}",
            style("[INFO] [").bold(),
            self.styles.get("<system>").unwrap().apply_to("<system>"),
            style("]").bold(),
            style(time).bold()
        );
        println!();
    }

    /// Logs that an agent started synchronizing a new agreement over [`Message`]s.
    ///
    /// # Arguments
    /// - `id`: The identifier of the agent who is logging.
    /// - `agrmnt`: The [`Agreement`] that will be added to the pool of agreements when synchronized.
    pub fn log_agree_start(&self, id: &str, agrmnt: &Agreement<Message>) {
        // Write the main log-line
        println!(
            "{}{}{} Initiated synchronization to agree on message '{}'",
            style("[INFO] [").bold(),
            self.styles.get(id).unwrap().apply_to(id),
            style("]").bold(),
            style(agrmnt.id()).bold()
        );

        // Write the set
        println!(" └> {}", agrmnt.display("Agreement", "    "));
        println!();
    }

    /// Logs that all agents agreed to synchronize time.
    ///
    /// # Arguments
    /// - `agrmnt`: The [`Agreement`] (over [`Message`]s) that will be added to the pool of agreements when synchronized.
    pub fn log_agree(&self, agrmnt: &Agreement<Message>) {
        // Write the main log-line
        println!(
            "{}{}{} New agreement '{}' created",
            style("[INFO] [").bold(),
            self.styles.get("<system>").unwrap().apply_to("<system>"),
            style("]").bold(),
            style(agrmnt.id()).bold()
        );

        // Write the set
        println!(" └> {}", agrmnt.display("Agreement", "    "));
        println!();
    }



    /// Logs an error message to stdout.
    ///
    /// # Arguments
    /// - `id`: The identifier of the agent who is logging.
    /// - `msg`: Some message (retrieved as [`Display`]) to show.
    pub fn error(&self, id: &str, msg: impl Display) {
        println!(
            "{}{}{}{}{} {}\n",
            style("[").bold(),
            style("ERROR").bold().red(),
            style("] [").bold(),
            self.styles.get(id).unwrap().apply_to(id),
            style("]").bold(),
            msg
        );
    }

    /// Logs a the result of a failed audit to stdout.
    ///
    /// # Arguments
    /// - `id`: The identifier of the agent who is logging.
    /// - `act`: The [`Action`] (over [`Message`]s) that failed the audit.
    /// - `expl`: The [`Explanation`] of why the audit of that action failed.
    pub fn error_audit<E1, E2>(&self, id: &str, act: &Action<Message>, expl: AuditExplanation<&str, E1, E2>) {
        // Write for that agent
        println!(
            "{}{}{}{}{} Action that enacts '{}' did not succeed audit",
            style("[").bold(),
            style("ERROR").bold().red(),
            style("] [").bold(),
            self.styles.get(id).unwrap().apply_to(id),
            style("]").bold(),
            act.enacts().id(),
        );

        // Retrieve the message IDs for the justication
        let just: LocalSet<&Message> = act.justification();
        let mut just_ids: String = String::new();
        for msg in just.iter() {
            if !just_ids.is_empty() {
                just_ids.push_str(", ");
            }
            just_ids.push_str(&msg.id().to_string());
        }

        // Generate serialized explanation
        let sexpl: String = match expl {
            AuditExplanation::Stated { stmt } => format!("Message '{}' is not stated", style(stmt).bold()),
            AuditExplanation::Extract { err: _ } => format!("Cannot extract policy"),
            AuditExplanation::Valid { expl: _ } => format!("Extracted policy is not valid"),
            AuditExplanation::Based { stmt } => format!("Message '{}' is not in the set of agreements", style(stmt).bold()),
            AuditExplanation::Timely { stmt, applies_at, taken_at } => format!(
                "Message '{}' is an agreement valid for time {}, but the action was taken at time {}",
                style(stmt).bold(),
                applies_at,
                taken_at
            ),
        };
        let sexpl: &str = sexpl.trim_end();

        // Write the sets
        print!(" ├> {}", act.basis().display("Basis", " |  "));
        print!(" ├> {}", just.display("Justification", " |  "));
        print!(" ├> {}", act.enacts().display("Enacts", " |  "));
        println!(" └> {sexpl}");
        println!();
    }
}
