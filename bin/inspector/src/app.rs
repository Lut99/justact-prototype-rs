//  APP.rs
//    by Lut99
//
//  Created:
//    16 Jan 2025, 12:18:55
//  Last edited:
//    16 Jan 2025, 16:51:46
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the main frontend app of the `inspector`.
//

use std::collections::VecDeque;
use std::ops::ControlFlow;
use std::sync::Arc;

use crossterm::event::EventStream;
use error_trace::trace;
use futures::{FutureExt as _, StreamExt as _};
use justact_prototype::io::Trace;
use log::{debug, error};
use parking_lot::{Mutex, MutexGuard};
use ratatui::Frame;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Style, Stylize as _};
use ratatui::widgets::{Block, List, ListState, Paragraph};
use thiserror::Error;
use tokio::io::AsyncRead;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::task::JoinHandle;

use crate::trace::TraceIter;


/***** ERRORS *****/
/// Defines the errors emitted by [`run()`].
#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to handle events from the terminal UI")]
    Event {
        #[source]
        err: std::io::Error,
    },
    #[error("Failed to render the terminal UI")]
    Render {
        #[source]
        err: std::io::Error,
    },
    #[error("Failed to get the next trace")]
    TraceRead {
        #[source]
        err: crate::trace::Error,
    },
}





/***** HELPERS *****/
/// Defines the UI windows to draw.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Window {
    /// The main window.
    Main,
}

/// Defines the state of the app.
///
/// This isn't worked on directly. Usually, it will be accessed through a `StateGuard` which has
/// access to locked fields.
#[derive(Debug)]
struct State {
    /// Which window we're currently drawing.
    window: Window,
    /// A queue of errors to show.
    errors: Arc<Mutex<VecDeque<Error>>>,
    /// The currently collected list of traces.
    traces: Arc<Mutex<Vec<Trace<'static>>>>,
    /// The currently selected trace.
    traces_state: ListState,
}
impl State {
    /// Constructor for the State that initializes it to default.
    ///
    /// # Arguments
    /// - `errors`: The shared queue of errors with the trace reader thread.
    /// - `traces`: The shared buffer of parsed [`Trace`]s with the trace reader thread.
    ///
    /// # Returns
    /// A new State reading for state'ing.
    fn new(errors: Arc<Mutex<VecDeque<Error>>>, traces: Arc<Mutex<Vec<Trace<'static>>>>) -> Self {
        Self { window: Window::Main, errors, traces, traces_state: ListState::default() }
    }

    /// Returns a [`StateGuard`] which has locks to the internal queue of errors and buffer of
    /// traces.
    ///
    /// # Returns
    /// A [`StateGuard`] which can be accessed.
    #[inline]
    fn lock(&mut self) -> StateGuard {
        StateGuard { window: &mut self.window, errors: self.errors.lock(), traces: self.traces.lock(), traces_state: &mut self.traces_state }
    }
}

/// Defines the accessible state of the app.
struct StateGuard<'s> {
    /// Which window we're currently drawing.
    window: &'s mut Window,
    /// A queue of errors to show.
    errors: MutexGuard<'s, VecDeque<Error>>,
    /// The currently collected list of traces.
    traces: MutexGuard<'s, Vec<Trace<'static>>>,
    /// The currently selected trace.
    traces_state: &'s mut ListState,
}





/***** LIBRARY *****/
/// The application UI, together with all its state.
#[derive(Debug)]
pub struct App {
    /// The app's state.
    state:    State,
    /// The [`EventStream`] used to receive events.
    events:   EventStream,
    /// The receiver channel used to receive redraw commands from the trace thread.
    receiver: Receiver<()>,
    /// The thread handle responsible for generating new traces.
    handle:   JoinHandle<()>,
}

// Constructors & Destructors
impl App {
    /// Creates a new App.
    ///
    /// # Arguments
    /// - `what`: Some name (path or otherwise) that describes the `input` (used for debugging purposes only).
    /// - `input`: Some [`Read`]er from which to read [`Trace`]s.
    ///
    /// # Returns
    /// An App that is ready for drawing.
    #[inline]
    pub fn new(what: impl Into<String>, input: impl 'static + Send + AsyncRead + Unpin) -> Self {
        let what: String = what.into();
        let errors = Arc::new(Mutex::new(VecDeque::new()));
        let traces = Arc::new(Mutex::new(Vec::new()));
        let (sender, receiver) = channel(3);
        Self {
            state: State::new(errors.clone(), traces.clone()),
            events: EventStream::new(),
            receiver,
            handle: tokio::spawn(Self::trace_reader(traces, errors, sender, what, input)),
        }
    }
}
impl Drop for App {
    fn drop(&mut self) {
        // Attempt to drop the handle
        self.handle.abort();
    }
}

// Game loop
impl App {
    /// Runs the application as a whole.
    ///
    /// It will consume the application. You'll have to start again once quit.
    ///
    /// # Errors
    /// This function can error if some I/O error occurred, either with the terminal window or
    /// stdout/the filesystem.
    pub async fn run(mut self) -> Result<(), Error> {
        let mut term = ratatui::init();
        loop {
            // Render the new UI state (immediate mode and all that)
            {
                log::trace!("Rendering terminal UI");
                let mut state: StateGuard = self.state.lock();
                if let Err(err) = term.draw(|frame| state.render(frame)) {
                    ratatui::restore();
                    return Err(Error::Render { err });
                }
            }

            // Handle any events
            tokio::select! {
                // The normal wait-for-events
                res = self.events.next().fuse() => {
                    match res {
                        Some(Ok(event)) => {
                            let mut state: StateGuard = self.state.lock();
                            match state.handle_event(event) {
                                Ok(ControlFlow::Continue(_)) => continue,
                                Ok(ControlFlow::Break(_)) => {
                                    ratatui::restore();
                                    return Ok(());
                                },
                                Err(err) => {
                                    ratatui::restore();
                                    return Err(err);
                                },
                            }
                        }
                        Some(Err(err)) => return Err(Error::Event { err }),
                        None => return Ok(()),
                    }
                },

                // The one that is used by the thread to trigger a redraw
                _ = self.receiver.recv() => {},
            };
        }
    }
}

// Events
impl<'s> StateGuard<'s> {
    /// Handles a event based on the current window.
    ///
    /// # Arguments
    /// - `event`: Some [`Event`] to handle.
    ///
    /// # Returns
    /// A [`ControlFlow`] describing whether the main game loop should
    /// [continue](ControlFlow::Continue) or [not](ControlFlow::Break).
    ///
    /// # Errors
    /// This function may error if we failed to handle them properly.
    fn handle_event(&mut self, event: Event) -> Result<ControlFlow<()>, Error> {
        log::trace!("Handling event {event:?} in {:?}", self.window);
        match &self.window {
            Window::Main => self.handle_event_main(event),
        }
    }

    /// Handles a event in the context of the main window.
    ///
    /// # Arguments
    /// - `event`: Some [`Event`] to handle.
    ///
    /// # Returns
    /// A [`ControlFlow`] describing whether the main game loop should
    /// [continue](ControlFlow::Continue) or [not](ControlFlow::Break).
    ///
    /// # Errors
    /// This function may error if we failed to handle them properly.
    fn handle_event_main(&mut self, event: Event) -> Result<ControlFlow<()>, Error> {
        match event {
            // (A)rrows
            Event::Key(KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Received key event UP");
                if !self.traces.is_empty() {
                    match self.traces_state.selected() {
                        Some(i) if i == 0 => self.traces_state.select(None),
                        Some(i) => self.traces_state.select(Some(i - 1)),
                        None => self.traces_state.select(Some(self.traces.len() - 1)),
                    }
                }
                Ok(ControlFlow::Continue(()))
            },
            Event::Key(KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Received key event DOWN");
                if !self.traces.is_empty() {
                    match self.traces_state.selected() {
                        Some(i) if i >= self.traces.len() - 1 => self.traces_state.select(None),
                        Some(i) => self.traces_state.select(Some(i + 1)),
                        None => self.traces_state.select(Some(0)),
                    }
                }
                Ok(ControlFlow::Continue(()))
            },

            // (Q)uit
            Event::Key(KeyEvent { code: KeyCode::Char('q'), modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Quitting...");
                Ok(ControlFlow::Break(()))
            },

            // Other events
            _ => Ok(ControlFlow::Continue(())),
        }
    }
}

// Rendering
impl<'s> StateGuard<'s> {
    /// Renders the application's current window.
    ///
    /// # Arguments
    /// - `frame`: Some [`Frame`] to render to.
    fn render(&mut self, frame: &mut Frame) {
        // Delegate to the appropriate window.
        match self.window {
            Window::Main => self.render_main(frame),
        }
    }

    /// Renders the application's main window.
    fn render_main(&mut self, frame: &mut Frame) {
        let rects = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3), Constraint::Fill(1)]).split(frame.area());

        // Title bar
        frame.render_widget(
            Paragraph::new(format!("JustAct Prototype Trace Inspector - v{}", env!("CARGO_PKG_VERSION")))
                .style(Style::new().bold())
                .block(Block::bordered()),
            rects[0],
        );

        // Traces
        let titles = self.traces.iter().map(|t| match t {
            Trace::AddAgreement { agree } => format!("Published agreement \"{} {}\"", agree.message.id.0, agree.message.id.1),
            Trace::AdvanceTime { timestamp } => format!("Advanced to time {timestamp}"),
            Trace::EnactAction { who, to: _, action } => format!("Agent {who:?} enacted action \"{} {}\"", action.id.0, action.id.1),
            Trace::StateMessage { who, to: _, msg } => format!("Agent {who:?} stated message \"{} {}\"", msg.id.0, msg.id.1),
        });
        frame.render_stateful_widget(
            List::new(titles).block(Block::bordered().title("Trace")).highlight_style(Style::new().bold()),
            rects[1],
            self.traces_state,
        );
    }
}

// Collecting traces
impl App {
    /// Thread that will push to the given list of traces once they become available.
    ///
    /// # Arguments
    /// - `output`: The [list](Vec) of [`Trace`]s to push to.
    /// - `errors`: A queue to push errors to.
    /// - `sender`: A [`Sender`] used to prompt redraws.
    /// - `what`: Some description of the `input`. Used for debugging only.
    /// - `input`: Some kind of [`Read`]able handle to read new [`Trace`]s from.
    ///
    /// # Returns
    /// This function will only return once the given `input` closes.
    async fn trace_reader(
        output: Arc<Mutex<Vec<Trace<'static>>>>,
        errors: Arc<Mutex<VecDeque<Error>>>,
        sender: Sender<()>,
        what: String,
        input: impl AsyncRead + Unpin,
    ) {
        // Simply iterate to add
        let mut stream = TraceIter::new(what.clone(), input);
        while let Some(trace) = stream.next().await {
            // Unwrap it to add
            match trace {
                Ok(trace) => {
                    debug!("Read trace {trace:?} from {what}");
                    {
                        let mut output: MutexGuard<Vec<Trace>> = output.lock();
                        output.push(trace);
                    }
                    // NOTE: We ignore the result, because it's just a redraw prompt anyway
                    let _ = sender.send(()).await;
                },
                Err(err) => {
                    error!("{}", trace!(("Failed to read trace from {what}"), err));
                    {
                        let mut errors: MutexGuard<VecDeque<Error>> = errors.lock();
                        errors.push_back(Error::TraceRead { err });
                    }
                    // NOTE: We ignore the result, because it's just a redraw prompt anyway
                    let _ = sender.send(()).await;
                },
            }
        }
    }
}
