//  APP.rs
//    by Lut99
//
//  Created:
//    16 Jan 2025, 12:18:55
//  Last edited:
//    16 Jan 2025, 17:30:42
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
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Style, Stylize as _};
use ratatui::text::{Span, Text};
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





/***** HELPER FUNCTIONS *****/
/// Centers an area for something.
///
/// # Arguments
/// - `horizontal`: Some [`Constraint`] for the horizontal space.
/// - `vertical`: Some [`Constraint`] for the vertical space.
/// - `area`: Some [`Rect`] that describes the full space to center in.
///
/// # Returns
/// A [`Rect`] that can make an element center.
fn center(horizontal: Constraint, vertical: Constraint, area: Rect) -> Rect {
    let [area] = Layout::horizontal([horizontal]).flex(Flex::Center).areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

/// Centers an area for some text.
///
/// # Arguments
/// - `text`: Some [`Text`] to center.
/// - `area`: Some [`Rect`] that describes the full space to center in.
///
/// # Returns
/// A [`Rect`] that can make an element center.
#[inline]
fn center_text(text: &Text, area: Rect) -> Rect { center(Constraint::Length(text.width() as u16), Constraint::Length(1), area) }

/// Renders some text centered in the given area.
///
/// # Arguments
/// - `frame`: The [`Frame`] to render in.
/// - `text`: Some [`Text`] to render.
/// - `area`: Some [`Rect`] that we render in.
#[inline]
fn render_centered_text(frame: &mut Frame, text: Text, area: Rect) {
    let area = center_text(&text, area);
    frame.render_widget(text, area);
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
    /// The currently opened trace.
    traces_opened: Option<usize>,
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
        Self { window: Window::Main, errors, traces, traces_state: ListState::default(), traces_opened: None }
    }

    /// Returns a [`StateGuard`] which has locks to the internal queue of errors and buffer of
    /// traces.
    ///
    /// # Returns
    /// A [`StateGuard`] which can be accessed.
    #[inline]
    fn lock(&mut self) -> StateGuard {
        StateGuard {
            window: &mut self.window,
            errors: self.errors.lock(),
            traces: self.traces.lock(),
            traces_state: &mut self.traces_state,
            traces_opened: &mut self.traces_opened,
        }
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
    /// The currently opened trace.
    traces_opened: &'s mut Option<usize>,
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
        let left_style = if self.traces_opened.is_some() { Style::new().dark_gray() } else { Style::new().white() };
        let vrects = Layout::vertical([Constraint::Length(3), Constraint::Fill(1), Constraint::Length(1)]).split(frame.area());

        // Title bar
        frame.render_widget(
            Paragraph::new(format!("JustAct Prototype Trace Inspector - v{}", env!("CARGO_PKG_VERSION")))
                .style(Style::new().bold())
                .block(Block::bordered()),
            vrects[0],
        );



        // Traces (left plane)
        let body_rects =
            Layout::horizontal(if self.traces_opened.is_some() { [Constraint::Fill(1); 2].as_slice() } else { [Constraint::Fill(1); 1].as_slice() })
                .split(vrects[1]);
        let titles = self.traces.iter().map(|t| match t {
            Trace::AddAgreement { agree } => {
                let mut text = Text::from("Published agreement ").style(left_style);
                text.push_span(Span::from(format!("Published agreement \"{} {}\"", agree.message.id.0, agree.message.id.1)).green());
                text
            },
            Trace::AdvanceTime { timestamp } => {
                let mut text = Text::from("Advanced to time ").style(left_style);
                text.push_span(Span::from(format!("{timestamp}")).cyan());
                text
            },
            Trace::EnactAction { who, to: _, action } => {
                let mut text = Text::from("Agent ").style(left_style);
                text.push_span(Span::from(format!("{who:?}")).bold());
                text.push_span(" enacted action ");
                text.push_span(Span::from(format!("\"{} {}\"", action.id.0, action.id.1)).yellow());
                text
            },
            Trace::StateMessage { who, to: _, msg } => {
                let mut text = Text::from("Agent ").style(left_style);
                text.push_span(Span::from(format!("{who:?}")).bold());
                text.push_span(" stated message ");
                text.push_span(Span::from(format!("\"{} {}\"", msg.id.0, msg.id.1)).red());
                text
            },
        });
        frame.render_stateful_widget(
            List::new(titles)
                .block(Block::bordered().title("Trace").style(left_style))
                .highlight_style(Style::new().fg(Color::Black).bg(if self.traces_opened.is_some() { Color::DarkGray } else { Color::White })),
            body_rects[0],
            self.traces_state,
        );



        // Opened trace (right plane)
        if let Some(i) = self.traces_opened {
            frame.render_widget(Paragraph::new("howdy").block(Block::bordered().title(format!("Trace {}", *i + 1))), body_rects[1]);
        }



        // Footer
        if self.traces_opened.is_some() {
            let hrects = Layout::horizontal([Constraint::Fill(1); 2].as_slice()).split(vrects[2]);

            render_centered_text(
                frame,
                {
                    let mut text = Text::from("Press ");
                    text.push_span(Span::from("Q").bold());
                    text.push_span(" to quit");
                    text
                },
                hrects[0],
            );
            render_centered_text(
                frame,
                {
                    let mut text = Text::from("Press ");
                    text.push_span(Span::from("Esc").bold());
                    text.push_span(" to return to all traces");
                    text
                },
                hrects[1],
            );
        } else {
            let hrects = Layout::horizontal(if self.traces_state.selected().is_some() {
                [Constraint::Fill(1); 4].as_slice()
            } else {
                [Constraint::Fill(1); 2].as_slice()
            })
            .split(vrects[2]);

            render_centered_text(
                frame,
                {
                    let mut text = Text::from("Press ");
                    if self.traces_state.selected().is_some() {
                        text.push_span(Span::from("Q").bold());
                    } else {
                        text.push_span(Span::from("Q").bold());
                        text.push_span("/");
                        text.push_span(Span::from("Esc").bold());
                    }
                    text.push_span(" to quit");
                    text
                },
                hrects[0],
            );
            if self.traces_state.selected().is_some() {
                render_centered_text(
                    frame,
                    {
                        let mut text = Text::from("Press ");
                        text.push_span(Span::from("Esc").bold());
                        text.push_span(" to unselect");
                        text
                    },
                    hrects[1],
                );
            }
            render_centered_text(
                frame,
                {
                    let mut text = Text::from("Press ");
                    text.push_span(Span::from("↑").bold());
                    text.push_span("/");
                    text.push_span(Span::from("↓").bold());
                    text.push_span(" to select traces");
                    text
                },
                hrects[if self.traces_state.selected().is_some() { 2 } else { 1 }],
            );
            if self.traces_state.selected().is_some() {
                render_centered_text(
                    frame,
                    {
                        let mut text = Text::from("Press ");
                        text.push_span(Span::from("Enter").bold());
                        text.push_span(" to view a trace");
                        text
                    },
                    hrects[3],
                );
            }
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
            // List management (Enter, Up, Down, Esc)
            Event::Key(KeyEvent { code: KeyCode::Enter, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Received key event ENTER");
                if self.traces_state.selected().is_some() {
                    // Make the currently selected one, opened
                    *self.traces_opened = self.traces_state.selected();
                }
                Ok(ControlFlow::Continue(()))
            },
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
            Event::Key(KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Received key event ESC");
                if self.traces_opened.is_some() {
                    *self.traces_opened = None;
                    Ok(ControlFlow::Continue(()))
                } else {
                    if self.traces_state.selected().is_some() {
                        self.traces_state.select(None);
                        Ok(ControlFlow::Continue(()))
                    } else {
                        debug!(target: "Main", "Quitting...");
                        Ok(ControlFlow::Break(()))
                    }
                }
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
