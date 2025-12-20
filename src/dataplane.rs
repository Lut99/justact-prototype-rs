//  StoreHandle.rs
//    by Lut99
//
//  Created:
//    21 Jan 2025, 11:01:12
//  Last edited:
//    31 Jan 2025, 18:12:55
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements an auxillary StoreHandle that generates StoreHandle-level
//!   traces.
//

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error;
use std::rc::Rc;

use thiserror::Error;

use crate::auditing::{Event, EventData};
use crate::io::EVENT_HANDLER;


/***** ERRORS *****/
/// Defines the errors originating from the [`StoreHandle`].
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to write the trace of something happening.
    #[error("Failed to handle trace with registered handler")]
    TraceHandle {
        #[source]
        err: Box<dyn 'static + Send + error::Error>,
    },
}





/***** LIBRARY *****/
/// Represents a [`StoreHandle`] but scoped to a particular agent.
#[derive(Debug)]
pub struct ScopedStoreHandle {
    /// The scoped StoreHandle.
    handle: StoreHandle,
    /// The agent who's it scoped to.
    agent:  String,
}

// Cloning
impl Clone for ScopedStoreHandle {
    #[inline]
    fn clone(&self) -> Self { Self { handle: StoreHandle(self.handle.0.clone()), agent: self.agent.clone() } }
}

// Operations
impl ScopedStoreHandle {
    /// Checks if a given variable has a value associated with it.
    ///
    /// # Arguments
    /// - `id`: The identifier (as a prefixed-by-author name) of the variable to check.
    ///
    /// # Returns
    /// True if the function exists, or false otherwise.
    #[inline]
    #[track_caller]
    pub fn exists(&self, id: &((String, String), String)) -> bool { self.handle.exists(id) }



    /// Reads the contents of a variable.
    ///
    /// Note that this returns the complete contents of the variable.
    ///
    /// # Arguments
    /// - `id`: The identifier (as a prefixed-by-author name) of the variable to read from.
    /// - `context`: The ID of an enacted action that is supposed to justify this write.
    ///
    /// # Returns
    /// A slice of bytes representing the dataset's contents, or [`None`] if the given variable
    /// never existed in the first place.
    ///
    /// # Errors
    /// This function can error if it failed to write a trace of what happened.
    #[inline]
    #[track_caller]
    pub fn read<'a>(
        &self,
        id: ((impl Into<String>, impl Into<String>), impl Into<String>),
        context: impl Into<Cow<'a, str>>,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.handle.read(&self.agent, id, context)
    }

    /// Writes the contents of a (new) variable.
    ///
    /// Note that this completely overwrites the contents of a dataset. Agents are responsible for
    /// reading, then writing the updated version if that behaviour is desired.
    ///
    /// # Arguments
    /// - `id`: The identifier (as a prefixed-by-author name) of the variable to write to.
    /// - `context`: The ID of an enacted action that is supposed to justify this write.
    /// - `contents`: Some bytes to write as payload.
    ///
    /// # Errors
    /// This function can error if it failed to write a trace of what happened.
    #[inline]
    #[track_caller]
    pub fn write<'a>(
        &self,
        id: ((impl Into<String>, impl Into<String>), impl Into<String>),
        context: impl Into<Cow<'a, str>>,
        contents: impl Into<Vec<u8>>,
    ) -> Result<(), Error> {
        self.handle.write(&self.agent, id, context, contents)
    }
}



/// Represents a virtual, in-memory variable <-> contents store for agents to play with data in.
///
/// This is used to model the real-world effects of the JustAct system.
///
/// Note that the StoreHandle acts as a handle. Thus, cloning it isn't possible; instead, you can
/// [scope](StoreHandle::scope()) it. Since the handles are done by shared pointers, you can safely
/// drop the original after all scopes have been made.
#[derive(Debug)]
pub struct StoreHandle(Rc<RefCell<HashMap<((String, String), String), Vec<u8>>>>);

// Constructors
impl Default for StoreHandle {
    #[inline]
    fn default() -> Self { Self::new() }
}
impl StoreHandle {
    /// Constructor for the StoreHandle.
    ///
    /// # Returns
    /// A new StoreHandle with no variables and no contents.
    #[inline]
    pub fn new() -> Self { Self(Rc::new(RefCell::new(HashMap::new()))) }
}

// Scoping
impl StoreHandle {
    /// Returns a [`ScopedStoreHandle`], which is like a shared handle but which will do all
    /// operations in the context of a particular agent.
    ///
    /// # Arguments
    /// - `agent`: The identifier of the agent to scope the StoreHandle handle to.
    ///
    /// # Returns
    /// A new [`ScopedStoreHandle`] that does the same as us but scoped.
    #[inline]
    pub fn scope(&self, agent: impl Into<String>) -> ScopedStoreHandle { ScopedStoreHandle { handle: Self(self.0.clone()), agent: agent.into() } }
}

// Operations
impl StoreHandle {
    /// Checks if a given variable has a value associated with it.
    ///
    /// # Arguments
    /// - `id`: The identifier (as a prefixed-by-author name) of the variable to check.
    ///
    /// # Returns
    /// True if the function exists, or false otherwise.
    #[inline]
    #[track_caller]
    pub fn exists(&self, id: &((String, String), String)) -> bool { self.0.borrow().contains_key(id) }



    /// Reads the contents of a variable.
    ///
    /// Note that this returns the complete contents of the variable.
    ///
    /// # Arguments
    /// - `who`: The agent who is reading the contents.
    /// - `id`: The identifier (as a prefixed-by-author name) of the variable to read from.
    /// - `context`: The ID of an enacted action that is supposed to justify this write.
    ///
    /// # Returns
    /// A slice of bytes representing the dataset's contents, or [`None`] if the given variable
    /// never existed in the first place.
    ///
    /// # Errors
    /// This function can error if it failed to write a trace of what happened.
    #[inline]
    #[track_caller]
    pub fn read<'a>(
        &self,
        who: impl AsRef<str>,
        id: ((impl Into<String>, impl Into<String>), impl Into<String>),
        context: impl Into<Cow<'a, str>>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let who: &str = who.as_ref();
        let id: ((String, String), String) = ((id.0.0.into(), id.0.1.into()), id.1.into());
        let context: Cow<'a, str> = context.into();

        // Perform the read
        let contents: Option<Vec<u8>> = { self.0.borrow().get(&id).cloned() };

        // Log it
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Data {
                event: EventData::Read {
                    who: who.into(),
                    id: Cow::Owned(id),
                    context,
                    contents: contents.as_ref().map(Vec::as_slice).map(Cow::Borrowed),
                },
            })
            .map_err(|err| Error::TraceHandle { err })?;

        // OK, return the contents
        Ok(contents)
    }

    /// Writes the contents of a (new) variable.
    ///
    /// Note that this completely overwrites the contents of a dataset. Agents are responsible for
    /// reading, then writing the updated version if that behaviour is desired.
    ///
    /// # Arguments
    /// - `who`: The agent who is writing the contents.
    /// - `id`: The identifier (as a prefixed-by-author name) of the variable to write to.
    /// - `context`: The ID of an enacted action that is supposed to justify this write.
    /// - `contents`: Some bytes to write as payload.
    ///
    /// # Errors
    /// This function can error if it failed to write a trace of what happened.
    #[inline]
    #[track_caller]
    pub fn write<'a>(
        &self,
        who: impl AsRef<str>,
        id: ((impl Into<String>, impl Into<String>), impl Into<String>),
        context: impl Into<Cow<'a, str>>,
        contents: impl Into<Vec<u8>>,
    ) -> Result<(), Error> {
        let who: &str = who.as_ref();
        let id: ((String, String), String) = ((id.0.0.into(), id.0.1.into()), id.1.into());
        let context: Cow<'a, str> = context.into();
        let contents: Vec<u8> = contents.into();

        // Log it first, for efficiency purposes (it can't fail anyway*)
        // * Famous last words
        let mut store = self.0.borrow_mut();
        EVENT_HANDLER
            .get()
            .unwrap_or_else(|| panic!("No trace handler was registered; call `register_trace_handler()` first"))
            .lock()
            .unwrap_or_else(|err| panic!("Lock poisoned: {err}"))
            .handle(Event::Data {
                event: EventData::Write {
                    who: Cow::Borrowed(who),
                    id: Cow::Borrowed(&id),
                    new: store.contains_key(&id),
                    context,
                    contents: Cow::Borrowed(&contents),
                },
            })
            .map_err(|err| Error::TraceHandle { err })?;

        // Perform the write and that's it
        store.insert(id, contents);
        Ok(())
    }
}
