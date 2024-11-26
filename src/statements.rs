//  STATEMENTS.rs
//    by Lut99
//
//  Created:
//    23 May 2024, 13:54:33
//  Last edited:
//    26 Nov 2024, 11:51:36
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements the local view set of stated- and enacted messages.
//

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FResult};
use std::rc::Rc;

use console::style;
use justact::auxillary::{Authored, Identifiable};
use justact::set::LocalSet;
use justact::statements::{Action, Message as JAMessage, Statements as JAStatements};

use crate::interface::{Displayable, Interface};


/***** FORMATTERS *****/
/// Writes [`Message`]s to some outgoing [`Formatter`].
pub struct MessageFormatter<'m, P, I> {
    /// The message to format.
    msg:    &'m Message,
    /// The prefix to write (e.g., `Message`).
    prefix: P,
    /// The indentation to write.
    indent: I,
}
impl<'m, P: Display, I: Display> Display for MessageFormatter<'m, P, I> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> FResult {
        // First, get the message's payload as UTF-8
        let spayload: Cow<str> = String::from_utf8_lossy(self.msg.payload());

        // Write the message
        writeln!(f, "{} '{}' by '{}' {{", self.prefix, style(self.msg.id()).bold(), style(self.msg.author()).bold())?;
        writeln!(f, "{}    {}", self.indent, spayload.replace('\n', &format!("\n{}    ", self.indent)).trim_end())?;
        writeln!(f, "{}}}", self.indent)
    }
}





/***** AUXILLARY *****/
/// Determines the possible targets that agents can send messages to for this [`Statements`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Target {
    /// Send it to everybody.
    All,
    /// Send it to a particular agent with this ID.
    Agent(String),
}
impl Target {
    /// Checks if this target matches all or not.
    ///
    /// # Returns
    /// True is `self` is [`Target::All`], or false otherwise.
    #[inline]
    pub fn is_all(&self) -> bool { matches!(self, Self::All) }

    /// Checks if the given agent is targeted by this Target.
    ///
    /// # Arguments
    /// - `agent`: The agent to check for a match.
    ///
    /// # Returns
    /// True if this Target targets the given `agent`, else false.
    #[inline]
    pub fn matches(&self, agent: &str) -> bool {
        match self {
            Self::All => true,
            Self::Agent(a) => *a == agent,
        }
    }
}





/***** LIBRARY *****/
/// Defines the prototype's notion of a message.
///
/// This means that it is assumed agents _cannot_ lie about their authorship of a message.
#[derive(Clone, Debug)]
pub struct Message {
    /// The identifier of the message.
    pub id:      String,
    /// The author of the message.
    pub author:  String,
    /// The payload of the message.
    pub payload: Vec<u8>,
}
impl Displayable for Message {
    type Formatter<'s, P: Display, I: Display> = MessageFormatter<'s, P, I> where Self: 's;

    #[inline]
    fn display<'s, P: Display, I: Display>(&'s self, prefix: P, indent: I) -> Self::Formatter<'s, P, I> {
        MessageFormatter { msg: self, prefix, indent }
    }
}

impl Identifiable for Message {
    type Id = str;

    #[inline]
    fn id(&self) -> &Self::Id { &self.id }
}
impl Authored for Message {
    type AuthorId = str;

    #[inline]
    fn author(&self) -> &Self::AuthorId { &self.author }
}
impl<'v> JAMessage<'v> for &'v Message {
    #[inline]
    fn id_v(&self) -> &'v Self::Id { &self.id }

    #[inline]
    fn author_v(&self) -> &'v Self::AuthorId { &self.author }

    #[inline]
    fn payload(&self) -> &'v [u8] { &self.payload }
}



/// An owned version of the statements.
///
/// Agents will see the agent-scoped variation [`Statements`].
#[derive(Debug)]
pub struct GlobalStatements {
    /// The current statements, scoped by agent.
    stmts: HashMap<String, LocalSet<Message>>,
    /// The current actions, scoped by agent.
    pub(crate) encts: HashMap<String, LocalSet<Action<Message>>>,
    /// An interface we use to log whatever happens in pretty ways.
    interface: Rc<RefCell<Interface>>,
}
impl GlobalStatements {
    /// Constructor for the GlobalStatements.
    ///
    /// # Arguments
    /// - `interface`: An interface we use to log whatever happens in pretty ways.
    ///
    /// # Returns
    /// A new GlobalStatements.
    #[inline]
    pub fn new(interface: Rc<RefCell<Interface>>) -> Self { Self { stmts: HashMap::new(), encts: HashMap::new(), interface } }

    /// Registers a new agent for target in the statements.
    ///
    /// Note that it will only receive _new_ statements emitted to all, not any sent before.
    ///
    /// # Arguments
    /// - `agent`: The new `A`gent to register.
    #[inline]
    pub fn register(&mut self, agent: impl Identifiable<Id = str>) {
        let id: &str = agent.id();
        self.stmts.insert(id.into(), LocalSet::new());
        self.encts.insert(id.into(), LocalSet::new());
    }

    /// Allows an agent scoped access to the Times-set.
    ///
    /// # Arguments
    /// - `agent`: The agent to scope this [`GlobalStatements`] for.
    /// - `func`: Some function that is executed for this scope.
    ///
    /// # Returns
    /// The result of the given closure `func`.
    #[inline]
    #[track_caller]
    pub fn scope<R>(&mut self, agent: &str, func: impl FnOnce(&mut Statements) -> R) -> R {
        // Call the closure
        let (res, mut stmts_queue, mut encts_queue): (R, Vec<(Target, Message)>, Vec<(Target, Action<Message>)>) = {
            let mut view = Statements {
                agent,
                stmts: self.stmts.get(agent).unwrap_or_else(|| panic!("Unknown given agent '{agent}'")),
                stmts_queue: vec![],
                encts: self.encts.get(agent).unwrap_or_else(|| panic!("Unknown given agent '{agent}'")),
                encts_queue: vec![],
            };
            let res: R = func(&mut view);
            (res, view.stmts_queue, view.encts_queue)
        };

        // Sync the changes back
        self.stmts.reserve(stmts_queue.len());
        self.encts.reserve(encts_queue.len());
        for (target, stmt) in stmts_queue.drain(..) {
            self.interface.borrow().log_state(agent, &stmt);
            if let Target::Agent(target) = target {
                self.stmts.get_mut(&target).unwrap_or_else(|| panic!("Unknown synchronize agent '{target}'")).add(stmt);
            } else {
                for stmts in self.stmts.values_mut() {
                    stmts.add(stmt.clone());
                }
            }
        }
        for (target, enct) in encts_queue.drain(..) {
            self.interface.borrow().log_enact(agent, &enct);
            if let Target::Agent(target) = target {
                self.encts.get_mut(&target).unwrap_or_else(|| panic!("Unknown synchronize agent '{target}'")).add(enct);
            } else {
                for encts in self.encts.values_mut() {
                    encts.add(enct.clone());
                }
            }
        }

        // OK, done
        res
    }
}
impl JAStatements for GlobalStatements {
    type Message = Message;
    type Target = Target;
    type Status = ();


    #[inline]
    #[track_caller]
    fn state(&mut self, target: Self::Target, msg: Self::Message) -> Self::Status {
        // Simply add directly
        match target {
            Target::All => {
                for msgs in self.stmts.values_mut() {
                    msgs.add(msg.clone());
                }
            },
            Target::Agent(agent) => {
                self.stmts.get_mut(&agent).map(|msgs| msgs.add(msg)).unwrap_or_else(|| panic!("Unknown agent '{agent}'"));
            },
        }
    }

    #[inline]
    fn stated<'s>(&'s self) -> LocalSet<&'s Self::Message> {
        // Build a set spanning all
        let mut set: LocalSet<&'s Message> = LocalSet::with_capacity(self.stmts.values().map(LocalSet::len).sum());
        for msgs in self.stmts.values() {
            set.extend(msgs);
        }
        set
    }



    #[inline]
    fn enact<'s>(&'s mut self, target: Self::Target, act: Action<Self::Message>) -> Self::Status {
        // Simply push to the queue
        match target {
            Target::All => {
                for acts in self.encts.values_mut() {
                    acts.add(act.clone());
                }
            },
            Target::Agent(agent) => {
                self.encts.get_mut(&agent).map(|acts| acts.add(act)).unwrap_or_else(|| panic!("Unknown agent '{agent}'"));
            },
        }
    }

    #[inline]
    fn enacted<'s>(&'s self) -> LocalSet<&'s Action<Self::Message>> {
        // Build a set spanning all
        let mut set: LocalSet<&'s Action<Message>> = LocalSet::with_capacity(self.encts.values().map(LocalSet::len).sum());
        for acts in self.encts.values() {
            set.extend(acts);
        }
        set
    }
}

/// Provides agents with a local view on the stated- and enacted messages.
#[derive(Debug)]
pub struct Statements<'v> {
    /// This agent
    agent: &'v str,

    /// The statements that this agent knows of.
    stmts: &'v LocalSet<Message>,
    /// A queue of statements that this agent pushed.
    pub(crate) stmts_queue: Vec<(Target, Message)>,

    /// The enactments that this agent knows of.
    encts: &'v LocalSet<Action<Message>>,
    /// A queue of enactments that this agent pushed.
    pub(crate) encts_queue: Vec<(Target, Action<Message>)>,
}
impl<'v> JAStatements for Statements<'v> {
    type Message = Message;
    type Target = Target;
    type Status = ();


    #[inline]
    #[track_caller]
    fn state(&mut self, target: Self::Target, msg: Self::Message) -> Self::Status {
        // Simply push to the queue
        self.stmts_queue.push((target, msg));
    }

    #[inline]
    fn stated<'s>(&'s self) -> LocalSet<&'s Self::Message> {
        // Start with what we know...
        let mut set: LocalSet<&'s Message> = self.stmts.iter().collect();
        // ...and push any queued items for us
        for (target, msg) in &self.stmts_queue {
            if target.matches(&self.agent) {
                set.add(msg.into());
            }
        }
        // OK
        set
    }



    #[inline]
    fn enact<'s>(&'s mut self, target: Self::Target, act: Action<Self::Message>) -> Self::Status {
        // Simply push to the queue
        self.encts_queue.push((target, act));
    }

    #[inline]
    fn enacted<'s>(&'s self) -> LocalSet<&'s Action<Self::Message>> {
        // Start with what we know...
        let mut set: LocalSet<&'s Action<Message>> = self.encts.iter().collect();
        // ...and push any queued items for us
        for (target, act) in &self.encts_queue {
            if target.matches(&self.agent) {
                set.add(act);
            }
        }
        // OK
        set
    }
}
impl<'s, 'v> JAStatements for &'s mut Statements<'v> {
    type Message = <Statements<'v> as JAStatements>::Message;
    type Target = <Statements<'v> as JAStatements>::Target;
    type Status = <Statements<'v> as JAStatements>::Status;

    #[inline]
    #[track_caller]
    fn state(&mut self, target: Self::Target, msg: Self::Message) -> Self::Status { Statements::state(self, target, msg) }

    #[inline]
    #[track_caller]
    fn stated<'s2>(&'s2 self) -> LocalSet<&'s2 Self::Message> { Statements::stated(self) }

    #[inline]
    #[track_caller]
    fn enact(&mut self, target: Self::Target, act: Action<Self::Message>) -> Self::Status { Statements::enact(self, target, act) }

    #[inline]
    #[track_caller]
    fn enacted<'s2>(&'s2 self) -> LocalSet<&'s2 Action<Self::Message>> { Statements::enacted(self) }
}
