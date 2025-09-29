//  APP.rs
//    by Lut99
//
//  Created:
//    16 Jan 2025, 12:18:55
//  Last edited:
//    30 Jan 2025, 21:05:12
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the main frontend app of the `inspector`.
//

use std::borrow::Cow;
use std::collections::VecDeque;
use std::fmt::Display;
use std::ops::ControlFlow;
use std::sync::Arc;

use crossterm::event::EventStream;
use error_trace::toplevel;
use futures::{FutureExt as _, StreamExt as _};
use justact::collections::Recipient;
use justact::collections::map::InfallibleMap;
use justact_prototype::auditing::{Audit, Event, EventControl, EventData, Permission};
use justact_prototype::policy::slick::{GroundAtom, Text as SlickText};
use justact_prototype::wire::Message;
use log::{debug, error};
use parking_lot::{Mutex, MutexGuard};
use ratatui::Frame;
use ratatui::crossterm::event::{Event as CEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Style, Stylize as _};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, List, ListState, Paragraph};
use thiserror::Error;
use tokio::io::AsyncRead;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::task::JoinHandle;

use crate::event_iter::EventIter;
use crate::widgets::scroll_area::{ScrollArea, ScrollState};


/***** ERRORS *****/
/// Defines the errors emitted by [`run()`].
#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get the next trace")]
    EventRead {
        #[source]
        err: crate::event_iter::Error,
    },
    #[error("Failed to render the terminal UI")]
    Render {
        #[source]
        err: std::io::Error,
    },
    #[error("Failed to handle events from the terminal UI")]
    TuiEvent {
        #[source]
        err: std::io::Error,
    },
}





/***** HELPER FUNCTIONS *****/
/// Will either wait on the given channel, or, if it's closed, wait indefinitely.
///
/// We do this because, otherwise, when the trace reading thread is closed, it will start
/// triggering redraws every CPU cycle instead of only on user events. Which is a waste.
///
/// # Arguments
/// - `channel`: The [`Receiver`] to wait for.
///
/// # Returns
/// [`Some(())`] if some message was received on the channel, or else [`None`].
///
/// Note this function NEVER returns if both [`channel.is_empty()`](Receiver::is_empty()) and
/// [`channel.is_closed()`](Receiver::is_closed()) are true.
#[inline]
async fn wait_for_event_or_forever(channel: &mut Receiver<()>) -> Option<()> {
    if !channel.is_empty() || !channel.is_closed() { channel.recv().await } else { std::future::pending().await }
}



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

/// Generates a [`Text`] for some button to press.
///
/// # Arguments
/// - `key`: The (textual representation of the) key to press.
/// - `what`: What happens when the key is pressed.
///
/// # Returns
/// A [`Text`] explaining to the user `what` happens when `key` is pressed.
fn press_to(key: impl Display, what: impl Display) -> Text<'static> {
    let mut text = Text::from("Press ");
    text.push_span(Span::from(key.to_string()).bold());
    text.push_span(format!(" to {what}"));
    text
}

/// Generates a [`Text`] for some button or another button to press.
///
/// # Arguments
/// - `key1`: The (textual representation of the) first key to press.
/// - `key2`: The (textual representation of the) other key to press.
/// - `what`: What happens when either key is pressed.
///
/// # Returns
/// A [`Text`] explaining to the user `what` happens when `key1` or `key2` is pressed.
fn press_or_to(key1: impl Display, key2: impl Display, what: impl Display) -> Text<'static> {
    let mut text = Text::from("Press ");
    text.push_span(Span::from(key1.to_string()).bold());
    text.push_span(format!("/"));
    text.push_span(Span::from(key2.to_string()).bold());
    text.push_span(format!(" to {what}"));
    text
}





/***** HELPERS *****/
/// Defines which part of the [main](Window::Main) window is focused.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Focus {
    /// The list of all trace is focused.
    List,
    /// The individual trace pane is focused.
    Event,
}

/// Defines the state of the app.
///
/// This isn't worked on directly. Usually, it will be accessed through a `StateGuard` which has
/// access to locked fields.
#[derive(Debug)]
struct State {
    /// A queue of errors to show.
    errors: Arc<Mutex<VecDeque<Error>>>,
    /// An audit happening live on the trace that provides us with validity.
    audit: Arc<Mutex<Audit>>,
    /// Which part of the window is focused.
    focus: Focus,
    /// The currently collected list of trace.
    trace: Arc<Mutex<Vec<Event<'static>>>>,
    /// The currently selected trace.
    selected_event: ListState,
    /// The currently opened trace.
    opened_event: Option<usize>,
    /// The scroll state of the right pane.
    right_scroll: ScrollState,
}
impl State {
    /// Constructor for the State that initializes it to default.
    ///
    /// # Arguments
    /// - `errors`: The shared queue of errors with the trace reader thread.
    /// - `trace`: The shared buffer of parsed [`Event`]s with the trace reader thread.
    /// - `audit`: Some shared [`Audit`] with the trace reader such that we can obtain action validities.
    ///
    /// # Returns
    /// A new State reading for state'ing.
    fn new(errors: Arc<Mutex<VecDeque<Error>>>, trace: Arc<Mutex<Vec<Event<'static>>>>, audit: Arc<Mutex<Audit>>) -> Self {
        Self {
            errors,
            trace,
            audit,
            focus: Focus::List,
            selected_event: ListState::default(),
            opened_event: None,
            right_scroll: ScrollState::default(),
        }
    }

    /// Returns a [`StateGuard`] which has locks to the internal queue of errors and buffer of
    /// trace.
    ///
    /// # Returns
    /// A [`StateGuard`] which can be accessed.
    #[inline]
    fn lock(&mut self) -> StateGuard {
        StateGuard {
            _errors: self.errors.lock(),
            audit: self.audit.lock(),
            focus: &mut self.focus,
            trace: self.trace.lock(),
            selected_event: &mut self.selected_event,
            opened_event: &mut self.opened_event,
            right_scroll: &mut self.right_scroll,
        }
    }
}

/// Defines the accessible state of the app.
struct StateGuard<'s> {
    /// A queue of errors to show.
    _errors: MutexGuard<'s, VecDeque<Error>>,
    /// An audit happening live on the trace that provides us with validity.
    audit: MutexGuard<'s, Audit>,
    /// Which part of the window is focused.
    focus: &'s mut Focus,
    /// The currently collected list of trace.
    trace: MutexGuard<'s, Vec<Event<'static>>>,
    /// The currently selected trace.
    selected_event: &'s mut ListState,
    /// The currently opened trace.
    opened_event: &'s mut Option<usize>,
    /// The scroll state of the right pane.
    right_scroll: &'s mut ScrollState,
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
    /// The thread handle responsible for generating new trace.
    handle:   JoinHandle<()>,
}

// Constructors & Destructors
impl App {
    /// Creates a new App.
    ///
    /// # Arguments
    /// - `what`: Some name (path or otherwise) that describes the `input` (used for debugging purposes only).
    /// - `input`: Some [`Read`]er from which to read [`Event`]s.
    ///
    /// # Returns
    /// An App that is ready for drawing.
    #[inline]
    pub fn new(what: impl Into<String>, input: impl 'static + Send + AsyncRead + Unpin) -> Self {
        let what: String = what.into();
        let errors = Arc::new(Mutex::new(VecDeque::new()));
        let trace = Arc::new(Mutex::new(Vec::new()));
        let audit = Arc::new(Mutex::new(Audit::new()));
        let (sender, receiver) = channel(3);
        Self {
            state: State::new(errors.clone(), trace.clone(), audit.clone()),
            events: EventStream::new(),
            receiver,
            handle: tokio::spawn(Self::trace_reader(errors, trace, audit, sender, what, input)),
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
                        Some(Err(err)) => return Err(Error::TuiEvent { err }),
                        None => return Ok(()),
                    }
                },

                // The one that is used by the thread to trigger a redraw
                _ = wait_for_event_or_forever(&mut self.receiver) => {},
            }
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
        let active = Color::White;
        let inactive = Color::DarkGray;
        let (left_color, right_color) = match *self.focus {
            Focus::List => (active, inactive),
            Focus::Event => (inactive, active),
        };
        let vrects = Layout::vertical([Constraint::Length(3), Constraint::Fill(1), Constraint::Length(1)]).split(frame.area());

        // Title bar
        frame.render_widget(
            Paragraph::new(format!("JustAct Prototype Event Inspector - v{}", env!("CARGO_PKG_VERSION")))
                .style(Style::new().bold())
                .block(Block::bordered()),
            vrects[0],
        );



        // Events (left plane)
        let max_trace_width: usize = (self.trace.len().checked_ilog10().unwrap_or(0) + 1) as usize;
        let body_rects =
            Layout::horizontal(if self.opened_event.is_some() { [Constraint::Fill(1); 2].as_slice() } else { [Constraint::Fill(1); 1].as_slice() })
                .split(vrects[1]);
        let titles = self.trace.iter().enumerate().map(|(i, t)| match t {
            Event::Control(t) => match t {
                EventControl::AddAgreement { agree } => {
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[JUSTACT]").italic());
                    text.push_span(" Published agreement ");
                    text.push_span(Span::from(format!("\"{} {}\"", agree.message.id.0, agree.message.id.1)).green());
                    text
                },
                EventControl::AdvanceTime { timestamp } => {
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[JUSTACT]").italic());
                    text.push_span(" Advanced to time ");
                    text.push_span(Span::from(format!("{timestamp}")).cyan());
                    text
                },
                EventControl::EnactAction { who, to: _, action } => {
                    // Then render
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[JUSTACT]").italic());
                    text.push_span(" Agent ");
                    text.push_span(Span::from(format!("{who}")).bold());
                    text.push_span(" enacted action ");
                    text.push_span(Span::from(format!("\"{} {}\"", action.id.0, action.id.1)).yellow());
                    text.push_span(" ");
                    text.push_span({
                        if self.audit.permission_of(i).and_then(|res| res.as_ref().map(|a| a.is_permitted()).ok()).unwrap_or(false) {
                            Span::from("✓").bold().green()
                        } else {
                            Span::from("✘").bold().white().on_red()
                        }
                    });
                    text
                },
                EventControl::StateMessage { who, to, msg } => {
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[JUSTACT]").italic());
                    text.push_span(" Agent ");
                    text.push_span(Span::from(format!("{who}")).bold());
                    text.push_span(" stated message ");
                    text.push_span(Span::from(format!("\"{} {}\"", msg.id.0, msg.id.1)).red());
                    if let Recipient::One(a) = to {
                        text.push_span(" to ");
                        text.push_span(Span::from(format!("{a}")).bold());
                    }
                    text
                },
            },

            Event::Data(t) => match t {
                EventData::Read { who, id, context, contents } => {
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[DATAPLN]").italic().black().bg(left_color));
                    text.push_span(" Agent ");
                    text.push_span(Span::from(format!("{who}")).bold());
                    text.push_span(" read variable ");
                    text.push_span(Span::from(format!("\"({} {}) {}\"", id.0.0, id.0.1, id.1)).bold().dark_gray());
                    text.push_span(" ");
                    match (
                        self.trace
                            .iter()
                            .position(|e| {
                                if let Event::Control(EventControl::EnactAction { action, .. }) = e {
                                    &(Cow::Borrowed(action.id.0.as_str()), action.id.1) == context
                                } else {
                                    false
                                }
                            })
                            .and_then(|i| self.audit.permission_of(i)),
                        contents.is_some(),
                    ) {
                        (Some(Ok(perm)), true) => {
                            if perm.is_permitted() {
                                text.push_span(Span::from("✓").bold().green());
                            } else {
                                text.push_span(Span::from("!!!").bold().white().on_red());
                            }
                        },
                        (_, _) => {
                            text.push_span(Span::from("!!!").bold().white().on_red());
                        },
                    }
                    text
                },
                EventData::Write { who, id, context, new, contents: _ } => {
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[DATAPLN]").italic().black().bg(left_color));
                    text.push_span(" Agent ");
                    text.push_span(Span::from(format!("{who}")).bold());
                    text.push_span(format!(" wrote to{} variable ", if *new { " new" } else { "" }));
                    text.push_span(Span::from(format!("\"({} {}) {}\"", id.0.0, id.0.1, id.1)).bold().dark_gray());
                    text.push_span(" ");
                    match self
                        .trace
                        .iter()
                        .position(|e| {
                            if let Event::Control(EventControl::EnactAction { action, .. }) = e {
                                &(Cow::Borrowed(action.id.0.as_str()), action.id.1) == context
                            } else {
                                false
                            }
                        })
                        .and_then(|i| self.audit.permission_of(i))
                    {
                        Some(Ok(perm)) => {
                            if perm.is_permitted() {
                                text.push_span(Span::from("✓").bold().green());
                            } else {
                                text.push_span(Span::from("!!!").bold().white().on_red());
                            }
                        },
                        _ => {
                            text.push_span(Span::from("!!!").bold().white().on_red());
                        },
                    }
                    text
                },
            },
        });
        frame.render_stateful_widget(
            List::new(titles).block(Block::bordered().title("Event").fg(left_color)).highlight_style(Style::new().fg(Color::Black).bg(left_color)),
            body_rects[0],
            self.selected_event,
        );



        // Opened trace (right plane)
        if let Some(i) = self.opened_event {
            let trace: &Event = &self.trace[*i];

            // Render the block
            let block = Block::bordered().title(format!("Event {}", *i + 1)).fg(right_color);
            frame.render_widget(&block, body_rects[1]);

            // Render the components
            match trace {
                Event::Control(trace) => match trace {
                    EventControl::AddAgreement { agree } => {
                        // Compute the size of the inner area of the scroll area
                        let text = Text::from(agree.message.payload.lines().map(|l| Line::raw(l)).collect::<Vec<Line>>());
                        let inner: Rect = Rect::new(0, 0, std::cmp::max(40, 2 + text.width() as u16), 4 + 2 + text.height() as u16);

                        // Render with the scroll area
                        frame.render_stateful_widget(
                            ScrollArea::new(inner).render_inner(move |mut frame| {
                                // Prepare the layout
                                let vrects = Layout::vertical(
                                    Some(Constraint::Length(1)).into_iter().cycle().take(4).chain(Some(Constraint::Length(2 + text.height() as u16))),
                                )
                                .split(frame.area());

                                // Render the ID & at times
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Agreement identifier: ");
                                        text.push_span(Span::from(format!("{} {}", agree.message.id.0, agree.message.id.1)).bold());
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[0],
                                );
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Agreement author    : ");
                                        text.push_span(Span::from(&agree.message.id.0).bold());
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[1],
                                );
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Agreement valid at  : ");
                                        text.push_span(Span::from(agree.at.to_string()).bold());
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[2],
                                );

                                // Render the payload
                                frame.render_widget(
                                    Paragraph::new(text).fg(right_color).block(Block::bordered().title("Payload").fg(right_color)),
                                    vrects[4],
                                );
                            }),
                            block.inner(body_rects[1]),
                            &mut self.right_scroll,
                        );
                    },
                    EventControl::AdvanceTime { timestamp } => {
                        // Render with the scroll area
                        let mut text = Text::from("Time advanced to: ");
                        text.push_span(Span::from(timestamp.to_string()).bold());
                        frame.render_stateful_widget(
                            ScrollArea::new(Rect::new(0, 0, text.width() as u16, text.height() as u16)).render_inner(|mut frame| {
                                // Render the time
                                frame.render_widget(Paragraph::new(text).fg(right_color), frame.area());
                            }),
                            block.inner(body_rects[1]),
                            &mut self.right_scroll,
                        );
                    },
                    EventControl::EnactAction { who, to, action } => {
                        // First, compute the denotation and decide if this was permitted
                        let denot: Result<(&Permission, Text<'static>), _> = self
                            .audit
                            .permission_of(*i)
                            .unwrap_or_else(|| {
                                panic!("Failed to find action {} \"{} {}\" in audit after list construction!", i, action.id.0, action.id.1)
                            })
                            .as_ref()
                            .map(|p| {
                                (p, {
                                    let mut text = Text::default();
                                    for t in &p.truths {
                                        let mut line = Line::from(format!("{t:?}"));
                                        if match &t {
                                            GroundAtom::Constant(t) if format!("{t:?}") == "error" => true,
                                            GroundAtom::Tuple(ts) if !ts.is_empty() && format!("{:?}", ts[0]) == "error" => true,
                                            _ => false,
                                        } {
                                            line = line.bold().white().on_red();
                                        }
                                        text.push_line(line);
                                    }
                                    text
                                })
                            });

                        // Then compute the total size of the needed inner area
                        let effect_height: usize = std::cmp::max(denot.as_ref().map(|(p, _)| p.effects.len()).unwrap_or(0), 1);
                        let (denot_width, denot_height): (u16, u16) =
                            denot.as_ref().map(|(_, text)| (2 + text.width() as u16, 2 + text.height() as u16)).unwrap_or((0, 0));
                        let inner: Rect = Rect::new(0, 0, std::cmp::max(40, denot_width), 13 + effect_height as u16 + denot_height);

                        // Render the information scrolled
                        frame.render_stateful_widget(
                            ScrollArea::new(inner).render_inner(|mut frame| {
                                let vrects = Layout::vertical(
                                    [Constraint::Length(1); 13]
                                        .into_iter()
                                        .chain([Constraint::Length(1)].into_iter().cycle().take(effect_height))
                                        .chain([Constraint::Length(denot_height)]),
                                )
                                .split(frame.area());

                                // Render who sent it to whom
                                let mut i: usize = 0;
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Enacted by: ");
                                        text.push_span(Span::from(who.as_ref()).bold());
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[i],
                                );
                                i += 1;
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Enacted to: ");
                                        text.push_span(
                                            Span::from(match to {
                                                Recipient::All => "<everyone>",
                                                Recipient::One(agent) => agent.as_ref(),
                                            })
                                            .bold(),
                                        );
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[i],
                                );
                                i += 2;

                                // Render the ID & at times
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Action identifier: ");
                                        text.push_span(Span::from(format!("{} {}", action.id.0, action.id.1)).bold());
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[i],
                                );
                                i += 1;
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Action actor     : ");
                                        text.push_span(Span::from(&action.id.0).bold());
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[i],
                                );
                                i += 1;
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Action taken at  : ");
                                        text.push_span(Span::from(action.basis.at.to_string()).bold());
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[i],
                                );
                                i += 2;

                                // Render the messages part of it
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Basis         : ");
                                        text.push_span(Span::from(format!("{} {}", action.basis.message.id.0, action.basis.message.id.1)).bold());
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[i],
                                );
                                i += 1;
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Justification : ");
                                        if !action.justification.is_empty() {
                                            let mut msgs: Vec<&Arc<Message>> = action.justification.iter().collect();
                                            msgs.sort_by(|lhs, rhs| lhs.id.0.cmp(&rhs.id.0).then_with(|| lhs.id.1.cmp(&rhs.id.1)));
                                            for (i, msg) in msgs.into_iter().enumerate() {
                                                if i > 0 && i < action.justification.len() - 1 {
                                                    text.push_span(", ");
                                                } else if i > 0 {
                                                    text.push_span(" and ");
                                                }
                                                text.push_span(Span::from(format!("{} {}", msg.id.0, msg.id.1)).bold());
                                            }
                                        } else {
                                            text.push_span(" <empty>");
                                        }
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[i],
                                );
                                i += 2;

                                // Render the interpretation part of it
                                match denot {
                                    Ok((perm, truths)) => {
                                        // Permission
                                        frame.render_widget(
                                            Paragraph::new({
                                                let mut text = Text::from("Permission : ");
                                                if perm.is_permitted() {
                                                    text.push_span(Span::from("OK").bold().green());
                                                } else {
                                                    text.push_span(Span::from("ILLEGAL").bold().red());
                                                    text.push_span(" (");
                                                    let mut first: bool = true;
                                                    if !perm.stated {
                                                        text.push_span(Span::from("not stated").red());
                                                        first = false;
                                                    }
                                                    if !perm.based {
                                                        if !first {
                                                            text.push_span(", ");
                                                        }
                                                        text.push_span(Span::from("not based").red());
                                                        first = false;
                                                    }
                                                    if !perm.valid {
                                                        if !first {
                                                            text.push_span(", ");
                                                        }
                                                        text.push_span(Span::from("not valid").red());
                                                        first = false;
                                                    }
                                                    if !perm.current {
                                                        if !first {
                                                            text.push_span(", ");
                                                        }
                                                        text.push_span(Span::from("not current").red());
                                                    }
                                                    text.push_span(")");
                                                }
                                                text
                                            })
                                            .fg(right_color),
                                            vrects[i],
                                        );
                                        i += 1;
                                        // Effects
                                        frame.render_widget(Paragraph::new("Effects    : ").fg(right_color), vrects[i]);
                                        i += 1;
                                        if !perm.effects.is_empty() {
                                            for effect in &perm.effects {
                                                frame.render_widget(
                                                    Paragraph::new({
                                                        let mut text = Text::from(" - ");
                                                        text.push_span(Span::from(format!("{:?}", effect.fact)).bold());
                                                        text
                                                    })
                                                    .fg(right_color),
                                                    vrects[i],
                                                );
                                                i += 1;
                                            }
                                        } else {
                                            frame.render_widget(Paragraph::new("   <none>").fg(right_color), vrects[i]);
                                            i += 1;
                                        }
                                        i += 1;

                                        // Finally, the denotation
                                        frame.render_widget(
                                            Paragraph::new(truths.clone())
                                                .block(Block::bordered().title("Justification truths").fg(right_color))
                                                .fg(right_color),
                                            vrects[i],
                                        );
                                    },
                                    Err(_) => todo!(),
                                }
                            }),
                            block.inner(body_rects[1]),
                            &mut self.right_scroll,
                        );
                    },
                    EventControl::StateMessage { who, to, msg } => {
                        // Compute the size of the total info area
                        let text = Text::from(msg.payload.lines().map(|l| Line::raw(l)).collect::<Vec<Line>>());
                        let inner: Rect = Rect::new(0, 0, std::cmp::max(40, 2 + text.width() as u16), 6 + 2 + text.height() as u16);

                        // Render in a scrolled area
                        frame.render_stateful_widget(
                            ScrollArea::new(inner).render_inner(|mut frame| {
                                // Prepare the layout
                                let vrects = Layout::vertical(
                                    Some(Constraint::Length(1)).into_iter().cycle().take(6).chain(Some(Constraint::Length(2 + text.height() as u16))),
                                )
                                .split(frame.area());

                                // Render who sent it to whom
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Stated by: ");
                                        text.push_span(Span::from(who.as_ref()).bold());
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[0],
                                );
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Stated to: ");
                                        text.push_span(
                                            Span::from(match to {
                                                Recipient::All => "<everyone>",
                                                Recipient::One(agent) => agent.as_ref(),
                                            })
                                            .bold(),
                                        );
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[1],
                                );

                                // Render the ID
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Message identifier: ");
                                        text.push_span(Span::from(format!("{} {}", msg.id.0, msg.id.1)).bold());
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[3],
                                );
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Message author    : ");
                                        text.push_span(Span::from(&msg.id.0).bold());
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[4],
                                );

                                // Render the basis payload
                                frame.render_widget(
                                    Paragraph::new(text).fg(right_color).block(Block::bordered().title("Payload").fg(right_color)),
                                    vrects[6],
                                );
                            }),
                            block.inner(body_rects[1]),
                            &mut self.right_scroll,
                        );
                    },
                },

                Event::Data(trace) => match trace {
                    EventData::Read { who, id, context, contents } => {
                        // Prepare the layout
                        let scontents: Option<Cow<str>> = contents.as_ref().map(Cow::as_ref).map(String::from_utf8_lossy);
                        let lines = scontents
                            .into_iter()
                            .map(|c| c.lines().map(|l| Line::raw(l.to_string())).collect::<Vec<Line>>())
                            .flatten()
                            .collect::<Vec<Line>>();
                        let lines = if !lines.is_empty() { lines } else { vec![Line::from("<no content>")] };
                        let text = Text::from(lines);
                        let perm: Result<&Permission, &str> = match self
                            .trace
                            .iter()
                            .position(|e| {
                                if let Event::Control(EventControl::EnactAction { action, .. }) = e {
                                    &(Cow::Borrowed(action.id.0.as_str()), action.id.1) == context
                                } else {
                                    false
                                }
                            })
                            .and_then(|i| self.audit.permission_of(i))
                        {
                            Some(Ok(perm)) => Ok(perm),
                            Some(Err(_)) => Err("FAILED TO EXTRACT POLICY!!!"),
                            None => Err("NOT FOUND!!!"),
                        };
                        let vrects = Layout::vertical(
                            [Constraint::Length(1); 5]
                                .into_iter()
                                .chain(if perm.is_ok() { Some(Constraint::Length(1)) } else { None }.into_iter())
                                .chain([Constraint::Length(2 + text.height() as u16)]),
                        )
                        .split(block.inner(body_rects[1]));

                        // Write the info first
                        frame.render_widget(
                            Paragraph::new({
                                let mut text = Text::from("Reader   : ");
                                text.push_span(Span::from(who.as_ref()).bold());
                                text
                            })
                            .fg(right_color),
                            vrects[0],
                        );
                        frame.render_widget(
                            Paragraph::new({
                                let mut text = Text::from("Variable : ");
                                text.push_span(Span::from(format!("({} {}) {}", id.0.0, id.0.1, id.1)).bold());
                                if contents.is_none() {
                                    text.push_span(" ");
                                    text.push_span(Span::from("NON-EXISTING!!!").bold().white().on_red());
                                }
                                text
                            })
                            .fg(right_color),
                            vrects[1],
                        );

                        // Render the context
                        frame.render_widget(
                            Paragraph::new({
                                let mut text = Text::from("Justified by : ");
                                text.push_span(Span::from(format!("{} {}", context.0, context.1)).yellow());
                                text.push_span(" ");
                                match perm {
                                    Ok(perm) => {
                                        if perm.is_permitted() {
                                            text.push_span(Span::from("✓").bold().green());
                                        } else {
                                            text.push_span(Span::from("✘").bold().white().on_red());
                                        }
                                    },
                                    Err(err) => {
                                        text.push_span(Span::from(err).bold().white().on_red());
                                    },
                                }
                                text
                            })
                            .fg(right_color),
                            vrects[3],
                        );
                        if let Ok(perm) = perm {
                            frame.render_widget(
                                Paragraph::new({
                                    let mut text = Text::from(" - Effect : ");
                                    let effect: GroundAtom = GroundAtom::Tuple(vec![
                                        GroundAtom::Constant(SlickText::from_str(context.0.as_ref())),
                                        GroundAtom::Constant(SlickText::from_str("reads")),
                                        GroundAtom::Tuple(vec![
                                            GroundAtom::Tuple(vec![
                                                GroundAtom::Constant(SlickText::from_str(&id.as_ref().0.0)),
                                                GroundAtom::Constant(SlickText::from_str(&id.as_ref().0.1)),
                                            ]),
                                            GroundAtom::Constant(SlickText::from_str(&id.as_ref().1)),
                                        ]),
                                    ]);
                                    text.push_span(Span::from(format!("{effect:?}")).bold());
                                    text.push_span(" ");
                                    if <[_]>::iter(&perm.effects).find(|e| e.fact == effect).is_some() {
                                        text.push_span(Span::from("✓").bold().green());
                                    } else {
                                        text.push_span(Span::from("NOT IN ACTION!!!").bold().white().on_red());
                                    }
                                    text
                                }),
                                vrects[4],
                            );
                        }

                        // Render the payload
                        if contents.is_some() {
                            frame.render_widget(
                                Paragraph::new(text).block(Block::bordered().title("Contents read").fg(right_color)).fg(right_color),
                                vrects[if perm.is_ok() { 6 } else { 5 }],
                            );
                        }
                    },
                    EventData::Write { who, id, context, new, contents } => {
                        // Prepare the layout
                        let scontents: Cow<str> = String::from_utf8_lossy(contents);
                        let lines = scontents.lines().map(|l| Line::raw(l.to_string())).collect::<Vec<Line>>();
                        let lines = if !lines.is_empty() { lines } else { vec![Line::from("<no content>")] };
                        let text = Text::from(lines);
                        let perm: Result<&Permission, &str> = match self
                            .trace
                            .iter()
                            .position(|e| {
                                if let Event::Control(EventControl::EnactAction { action, .. }) = e {
                                    &(Cow::Borrowed(action.id.0.as_str()), action.id.1) == context
                                } else {
                                    false
                                }
                            })
                            .and_then(|i| self.audit.permission_of(i))
                        {
                            Some(Ok(perm)) => Ok(perm),
                            Some(Err(_)) => Err("FAILED TO EXTRACT POLICY!!!"),
                            None => Err("NOT FOUND!!!"),
                        };
                        let vrects = Layout::vertical(
                            [Constraint::Length(1); 5]
                                .into_iter()
                                .chain(if perm.is_ok() { Some(Constraint::Length(1)) } else { None }.into_iter())
                                .chain([Constraint::Length(2 + text.height() as u16)]),
                        )
                        .split(block.inner(body_rects[1]));

                        // Write the info first
                        frame.render_widget(
                            Paragraph::new({
                                let mut text = Text::from("Writer   : ");
                                text.push_span(Span::from(who.as_ref()).bold());
                                text
                            })
                            .fg(right_color),
                            vrects[0],
                        );
                        frame.render_widget(
                            Paragraph::new({
                                let mut text = Text::from("Variable : ");
                                text.push_span(Span::from(format!("({} {}) {}", id.0.0, id.0.1, id.1)).bold());
                                if *new {
                                    text.push_span(" ");
                                    text.push_span(Span::from("(NEW)").bold().cyan());
                                }
                                text
                            })
                            .fg(right_color),
                            vrects[1],
                        );

                        // Render the context
                        frame.render_widget(
                            Paragraph::new({
                                let mut text = Text::from("Justified by : ");
                                text.push_span(Span::from(format!("{} {}", context.0, context.1)).yellow());
                                text.push_span(" ");
                                match perm {
                                    Ok(perm) => {
                                        if perm.is_permitted() {
                                            text.push_span(Span::from("✓").bold().green());
                                        } else {
                                            text.push_span(Span::from("✘").bold().white().on_red());
                                        }
                                    },
                                    Err(err) => {
                                        text.push_span(Span::from(err).bold().white().on_red());
                                    },
                                }
                                text
                            })
                            .fg(right_color),
                            vrects[3],
                        );
                        if let Ok(perm) = perm {
                            frame.render_widget(
                                Paragraph::new({
                                    let mut text = Text::from(" - Effect : ");
                                    let effect: GroundAtom = GroundAtom::Tuple(vec![
                                        GroundAtom::Constant(SlickText::from_str(context.0.as_ref())),
                                        GroundAtom::Constant(SlickText::from_str("writes")),
                                        GroundAtom::Tuple(vec![
                                            GroundAtom::Tuple(vec![
                                                GroundAtom::Constant(SlickText::from_str(&id.as_ref().0.0)),
                                                GroundAtom::Constant(SlickText::from_str(&id.as_ref().0.1)),
                                            ]),
                                            GroundAtom::Constant(SlickText::from_str(&id.as_ref().1)),
                                        ]),
                                    ]);
                                    text.push_span(Span::from(format!("{effect:?}")).bold());
                                    text.push_span(" ");
                                    if <[_]>::iter(&perm.effects).find(|e| e.fact == effect).is_some() {
                                        text.push_span(Span::from("✓").bold().green());
                                    } else {
                                        text.push_span(Span::from("NOT IN ACTION!!!").bold().white().on_red());
                                    }
                                    text
                                }),
                                vrects[4],
                            );
                        }

                        // Render the payload
                        frame.render_widget(
                            Paragraph::new(text).block(Block::bordered().title("Contents written").fg(right_color)).fg(right_color),
                            vrects[if perm.is_ok() { 6 } else { 5 }],
                        );
                    },
                },
            }
        }



        // Footer
        if *self.focus == Focus::Event {
            let hrects = Layout::horizontal([Constraint::Fill(1); 3].as_slice()).split(vrects[2]);

            render_centered_text(frame, press_to("Q", "quit"), hrects[0]);
            render_centered_text(frame, press_to("Esc", "close event"), hrects[1]);
            render_centered_text(frame, press_or_to("Shift+←", "Tab", "switch to list"), hrects[2]);
        } else {
            let n_boxes: usize = 2 + self.selected_event.selected().map(|_| 2).unwrap_or(0) + self.opened_event.map(|_| 1).unwrap_or(0);
            let hrects = Layout::horizontal(Some(Constraint::Fill(1)).into_iter().cycle().take(n_boxes)).split(vrects[2]);

            let mut i: usize = 0;
            render_centered_text(
                frame,
                if self.selected_event.selected().is_some() { press_or_to("Q", "Esc", "quit") } else { press_to("Q", "quit") },
                hrects[i],
            );
            i += 1;
            if self.selected_event.selected().is_some() {
                render_centered_text(frame, press_to("Esc", "unselect"), hrects[i]);
                i += 1;
            }
            if self.opened_event.is_some() {
                render_centered_text(frame, press_or_to("Shift+→", "Tab", "switch to event"), hrects[i]);
                i += 1;
            }
            render_centered_text(frame, press_or_to("↑", "↓", "select event"), hrects[i]);
            i += 1;
            if self.selected_event.selected().is_some() {
                render_centered_text(frame, press_to("Enter", "view an event"), hrects[i]);
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
    fn handle_event(&mut self, event: CEvent) -> Result<ControlFlow<()>, Error> {
        match event {
            // List management (Enter, Up, Down, Esc)
            CEvent::Key(KeyEvent { code: KeyCode::Enter, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Received key event ENTER");
                if *self.focus == Focus::List && self.selected_event.selected().is_some() {
                    // Make the currently selected one, opened
                    *self.opened_event = self.selected_event.selected();
                    *self.focus = Focus::Event;
                    self.right_scroll.reset();
                }
                Ok(ControlFlow::Continue(()))
            },
            CEvent::Key(KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Received key event UP");
                if !self.trace.is_empty() && *self.focus == Focus::List {
                    match self.selected_event.selected() {
                        Some(i) if i == 0 => self.selected_event.select(None),
                        Some(i) => self.selected_event.select(Some(i - 1)),
                        None => self.selected_event.select(Some(self.trace.len() - 1)),
                    }
                    // Also update the opened one if any
                    if self.opened_event.is_some() {
                        *self.opened_event = self.selected_event.selected();
                        self.right_scroll.reset();
                        if self.opened_event.is_none() {
                            *self.focus = Focus::List;
                        }
                    }
                } else if *self.focus == Focus::Event {
                    self.right_scroll.scroll_up();
                }
                Ok(ControlFlow::Continue(()))
            },
            CEvent::Key(KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Received key event DOWN");
                if !self.trace.is_empty() && *self.focus == Focus::List {
                    match self.selected_event.selected() {
                        Some(i) if i >= self.trace.len() - 1 => self.selected_event.select(None),
                        Some(i) => self.selected_event.select(Some(i + 1)),
                        None => self.selected_event.select(Some(0)),
                    }
                    // Also update the opened one if any
                    if self.opened_event.is_some() {
                        *self.opened_event = self.selected_event.selected();
                        self.right_scroll.reset();
                        if self.opened_event.is_none() {
                            *self.focus = Focus::List;
                        }
                    }
                } else if *self.focus == Focus::Event {
                    self.right_scroll.scroll_down();
                }
                Ok(ControlFlow::Continue(()))
            },
            CEvent::Key(KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                if *self.focus == Focus::Event {
                    self.right_scroll.scroll_left();
                }
                Ok(ControlFlow::Continue(()))
            },
            CEvent::Key(KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                if *self.focus == Focus::Event {
                    self.right_scroll.scroll_right();
                }
                Ok(ControlFlow::Continue(()))
            },
            CEvent::Key(KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Received key event ESC");
                if *self.focus == Focus::List {
                    if self.selected_event.selected().is_some() {
                        self.selected_event.select(None);
                        *self.opened_event = None;
                        *self.focus = Focus::List;
                        Ok(ControlFlow::Continue(()))
                    } else {
                        debug!(target: "Main", "Quitting...");
                        Ok(ControlFlow::Break(()))
                    }
                } else {
                    *self.opened_event = None;
                    *self.focus = Focus::List;
                    Ok(ControlFlow::Continue(()))
                }
            },

            // Focus management
            CEvent::Key(KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::SHIFT, kind: KeyEventKind::Press, state: _ })
            | CEvent::Key(KeyEvent { code: KeyCode::Tab, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ })
                if *self.focus == Focus::List =>
            {
                // If it's opened, we can shift
                if self.opened_event.is_some() {
                    *self.focus = Focus::Event;
                }
                Ok(ControlFlow::Continue(()))
            },
            CEvent::Key(KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::SHIFT, kind: KeyEventKind::Press, state: _ })
            | CEvent::Key(KeyEvent { code: KeyCode::Tab, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ })
                if *self.focus == Focus::Event =>
            {
                // If it's opened, we can shift
                if self.opened_event.is_some() {
                    *self.focus = Focus::List;
                }
                Ok(ControlFlow::Continue(()))
            },

            // (Q)uit
            CEvent::Key(KeyEvent { code: KeyCode::Char('q'), modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Quitting...");
                Ok(ControlFlow::Break(()))
            },

            // Other events
            _ => Ok(ControlFlow::Continue(())),
        }
    }
}

// Collecting trace
impl App {
    /// Thread that will push to the given list of trace once they become available.
    ///
    /// # Arguments
    /// - `errors`: A queue to push errors to.
    /// - `output`: The [list](Vec) of [`Event`]s to push to.
    /// - `audit`: A shared, running audit that is used to cache validity of actions as they come
    ///   in.
    /// - `sender`: A [`Sender`] used to prompt redraws.
    /// - `what`: Some description of the `input`. Used for debugging only.
    /// - `input`: Some kind of [`Read`]able handle to read new [`Event`]s from.
    ///
    /// # Returns
    /// This function will only return once the given `input` closes.
    async fn trace_reader(
        errors: Arc<Mutex<VecDeque<Error>>>,
        output: Arc<Mutex<Vec<Event<'static>>>>,
        audit: Arc<Mutex<Audit>>,
        sender: Sender<()>,
        what: String,
        input: impl AsyncRead + Unpin,
    ) {
        // Simply iterate over the input stream to collect trace
        let mut stream = EventIter::new(what.clone(), input);
        while let Some(event) = stream.next().await {
            // Unwrap it
            match event {
                Ok(event) => {
                    debug!("Read event {event:?} from {what}");

                    // Perform an audit on the trace
                    {
                        let mut audit: MutexGuard<Audit> = audit.lock();
                        audit.audit(&event);
                    }

                    // Add the trace to the output
                    {
                        let mut output: MutexGuard<Vec<Event>> = output.lock();
                        output.push(event);
                    }

                    // NOTE: We ignore the result of polling the interface to redraw, because worst
                    //       case, it simply won't be redrawn
                    let _ = sender.send(()).await;
                },
                Err(err) => {
                    error!("{}", toplevel!(("Failed to read event from {what}"), err));

                    // Add the event to the output
                    {
                        let mut errors: MutexGuard<VecDeque<Error>> = errors.lock();
                        errors.push_back(Error::EventRead { err });
                    }

                    // NOTE: We ignore the result of polling the interface to redraw, because worst
                    //       case, it simply won't be redrawn
                    let _ = sender.send(()).await;
                },
            }
        }
    }
}
