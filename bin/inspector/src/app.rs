//  APP.rs
//    by Lut99
//
//  Created:
//    16 Jan 2025, 12:18:55
//  Last edited:
//    23 Jan 2025, 15:13:45
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines the main frontend app of the `inspector`.
//

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::fmt::Display;
use std::ops::ControlFlow;
use std::sync::Arc;

use crossterm::event::EventStream;
use error_trace::trace;
use futures::{FutureExt as _, StreamExt as _};
use justact::auxillary::Actored;
use justact::collections::Selector;
use justact::collections::map::InfallibleMap;
use justact::collections::set::InfallibleSet;
use justact::policies::Policy as _;
use justact_prototype::io::{Trace, TraceDataplane, TraceJustAct};
use justact_prototype::policy::slick::{AffectorAtom, Denotation, Effect, Extractor, GroundAtom, PatternAtom, SyntaxError, Text as SlickText};
use justact_prototype::wire::Message;
use log::{debug, error};
use parking_lot::{Mutex, MutexGuard};
use ratatui::Frame;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Style, Stylize as _};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, List, ListState, Paragraph};
use thiserror::Error;
use tokio::io::AsyncRead;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::task::JoinHandle;

use crate::trace::TraceIter;
use crate::widgets::scroll_area::{ScrollArea, ScrollState};


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
/// Defines the UI windows to draw.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Window {
    /// The main window.
    Main,
}

/// Defines which part of the [main](Window::Main) window is focused.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Focus {
    /// The list of all traces is focused.
    List,
    /// The individual trace pane is focused.
    Trace,
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
    /// Which part of the window is focused.
    focus: Focus,
    /// The currently collected list of traces.
    traces: Arc<Mutex<Vec<Trace<'static>>>>,
    /// The currently selected trace.
    traces_state: ListState,
    /// The currently opened trace.
    traces_opened: Option<usize>,
    /// The horizontal scroll state of the right pane.
    right_hscroll: ScrollState,
    /// The vertical scroll state of the right pane.
    right_vscroll: ScrollState,

    /// A cache for computed denotations of actions.
    denot_cache: HashMap<(String, u32), Result<(Denotation, Text<'static>), SyntaxError>>,
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
        Self {
            window: Window::Main,
            errors,
            traces,
            focus: Focus::List,
            traces_state: ListState::default(),
            traces_opened: None,
            right_hscroll: ScrollState::default(),
            right_vscroll: ScrollState::default(),

            denot_cache: HashMap::with_capacity(4),
        }
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
            _errors: self.errors.lock(),
            focus: &mut self.focus,
            traces: self.traces.lock(),
            traces_state: &mut self.traces_state,
            traces_opened: &mut self.traces_opened,
            right_hscroll: &mut self.right_hscroll,
            right_vscroll: &mut self.right_vscroll,

            denot_cache: &mut self.denot_cache,
        }
    }
}

/// Defines the accessible state of the app.
struct StateGuard<'s> {
    /// Which window we're currently drawing.
    window: &'s mut Window,
    /// A queue of errors to show.
    _errors: MutexGuard<'s, VecDeque<Error>>,
    /// Which part of the window is focused.
    focus: &'s mut Focus,
    /// The currently collected list of traces.
    traces: MutexGuard<'s, Vec<Trace<'static>>>,
    /// The currently selected trace.
    traces_state: &'s mut ListState,
    /// The currently opened trace.
    traces_opened: &'s mut Option<usize>,
    /// The horizontal scroll state of the right pane.
    right_hscroll: &'s mut ScrollState,
    /// The vertical scroll state of the right pane.
    right_vscroll: &'s mut ScrollState,

    /// A cache for computed denotations of actions.
    denot_cache: &'s mut HashMap<(String, u32), Result<(Denotation, Text<'static>), SyntaxError>>,
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
            handle: tokio::spawn(Self::trace_reader(errors, traces, sender, what, input)),
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
        // Delegate to the appropriate window.
        match self.window {
            Window::Main => self.render_main(frame),
        }
    }

    /// Renders the application's main window.
    fn render_main(&mut self, frame: &mut Frame) {
        let active = Color::White;
        let inactive = Color::DarkGray;
        let (left_color, right_color) = match *self.focus {
            Focus::List => (active, inactive),
            Focus::Trace => (inactive, active),
        };
        let vrects = Layout::vertical([Constraint::Length(3), Constraint::Fill(1), Constraint::Length(1)]).split(frame.area());

        // Title bar
        frame.render_widget(
            Paragraph::new(format!("JustAct Prototype Trace Inspector - v{}", env!("CARGO_PKG_VERSION")))
                .style(Style::new().bold())
                .block(Block::bordered()),
            vrects[0],
        );



        // Traces (left plane)
        let max_trace_width: usize = (self.traces.len().checked_ilog10().unwrap_or(0) + 1) as usize;
        let body_rects =
            Layout::horizontal(if self.traces_opened.is_some() { [Constraint::Fill(1); 2].as_slice() } else { [Constraint::Fill(1); 1].as_slice() })
                .split(vrects[1]);
        let titles = self.traces.iter().enumerate().map(|(i, t)| match t {
            Trace::JustAct(t) => match t {
                TraceJustAct::AddAgreement { agree } => {
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[JUSTACT]").italic());
                    text.push_span(" Published agreement ");
                    text.push_span(Span::from(format!("\"{} {}\"", agree.message.id.0, agree.message.id.1)).green());
                    text
                },
                TraceJustAct::AdvanceTime { timestamp } => {
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[JUSTACT]").italic());
                    text.push_span(" Advanced to time ");
                    text.push_span(Span::from(format!("{timestamp}")).cyan());
                    text
                },
                TraceJustAct::EnactAction { who, to: _, action } => {
                    // Before we render, update the denotation cache
                    let denot: Result<&Denotation, _> = match self.denot_cache.get(&action.id) {
                        Some(res) => res.as_ref().map(|(d, _)| d),
                        None => {
                            // Compute the denotation (with a usecase-specific effect pattern)
                            let denot: Result<Denotation, _> =
                                Extractor.extract_with_actor(action.actor_id(), &action.justification).map(|mut pol| {
                                    pol.update_effect_pattern(
                                        PatternAtom::Tuple(vec![
                                            PatternAtom::Variable(SlickText::from_str("Worker")),
                                            PatternAtom::ConstantSet(vec![SlickText::from_str("reads"), SlickText::from_str("writes")]),
                                            PatternAtom::Variable(SlickText::from_str("Variable")),
                                        ]),
                                        AffectorAtom::Variable(SlickText::from_str("Worker")),
                                    );
                                    pol.truths()
                                });

                            // Then we serialize the denotation into a `Text` of effects, already ordered by error
                            let denot = denot.map(|d| {
                                let mut truths: Vec<(bool, String)> = <Denotation as InfallibleSet<GroundAtom>>::iter(&d)
                                    .map(|t| {
                                        (
                                            match t {
                                                GroundAtom::Constant(t) if format!("{t:?}") == "error" => true,
                                                GroundAtom::Tuple(ts) if !ts.is_empty() && format!("{:?}", ts[0]) == "error" => true,
                                                _ => false,
                                            },
                                            format!("{t:?}"),
                                        )
                                    })
                                    .collect();
                                truths.sort_by(|lhs, rhs| match (lhs.0, rhs.0) {
                                    (true, false) => Ordering::Less,
                                    (false, true) => Ordering::Greater,
                                    (true, true) | (false, false) => lhs.1.cmp(&rhs.1),
                                });
                                (
                                    d,
                                    Text::from(
                                        truths
                                            .into_iter()
                                            .map(|(is_err, line)| if is_err { Line::from(line).bold().red() } else { Line::from(line) })
                                            .collect::<Vec<Line>>(),
                                    ),
                                )
                            });

                            // Cache it
                            self.denot_cache.insert(action.id.clone(), denot);
                            self.denot_cache.get(&action.id).unwrap().as_ref().map(|(d, _)| d)
                        },
                    };

                    // Then render
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[JUSTACT]").italic());
                    text.push_span(" Agent ");
                    text.push_span(Span::from(format!("{who}")).bold());
                    text.push_span(" enacted action ");
                    text.push_span(Span::from(format!("\"{} {}\"", action.id.0, action.id.1)).yellow());
                    text.push_span(" ");
                    text.push_span(if denot.map(|d| d.is_valid()).unwrap_or(false) {
                        Span::from("✓").bold().green()
                    } else {
                        Span::from("✘").bold().white().on_red()
                    });
                    text
                },
                TraceJustAct::StateMessage { who, to, msg } => {
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[JUSTACT]").italic());
                    text.push_span(" Agent ");
                    text.push_span(Span::from(format!("{who}")).bold());
                    text.push_span(" stated message ");
                    text.push_span(Span::from(format!("\"{} {}\"", msg.id.0, msg.id.1)).red());
                    if let Selector::Agent(a) = to {
                        text.push_span(" to ");
                        text.push_span(Span::from(format!("{a}")).bold());
                    }
                    text
                },
            },

            Trace::Dataplane(t) => match t {
                TraceDataplane::Read { who, id, contents } => {
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[DATAPLN]").italic().black().bg(left_color));
                    text.push_span(" Agent ");
                    text.push_span(Span::from(format!("{who}")).bold());
                    text.push_span(" read variable ");
                    text.push_span(Span::from(format!("\"({} {}) {}\"", id.0.0, id.0.1, id.1)).bold().dark_gray());
                    if contents.is_none() {
                        text.push_span(" ");
                        text.push_span(Span::from("!!!").white().on_red());
                    }
                    text
                },
                TraceDataplane::Write { who, id, new, contents: _ } => {
                    let mut text = Text::default().fg(left_color);
                    text.push_span(Span::from(format!("{:>max_trace_width$}) ", i + 1)).dark_gray());
                    text.push_span(Span::from("[DATAPLN]").italic().black().bg(left_color));
                    text.push_span(" Agent ");
                    text.push_span(Span::from(format!("{who}")).bold());
                    text.push_span(format!(" wrote to{} variable ", if *new { " new" } else { "" }));
                    text.push_span(Span::from(format!("\"({} {}) {}\"", id.0.0, id.0.1, id.1)).bold().dark_gray());
                    text
                },
            },
        });
        frame.render_stateful_widget(
            List::new(titles).block(Block::bordered().title("Trace").fg(left_color)).highlight_style(Style::new().fg(Color::Black).bg(left_color)),
            body_rects[0],
            self.traces_state,
        );



        // Opened trace (right plane)
        if let Some(i) = self.traces_opened {
            let trace: &Trace = &self.traces[*i];

            // Render the block
            let block = Block::bordered().title(format!("Trace {}", *i + 1)).fg(right_color);
            frame.render_widget(&block, body_rects[1]);

            // Render the components
            match trace {
                Trace::JustAct(trace) => match trace {
                    TraceJustAct::AddAgreement { agree } => {
                        // Prepare the layout
                        let text = Text::from(agree.message.payload.lines().map(|l| Line::raw(l)).collect::<Vec<Line>>());
                        let vrects = Layout::vertical(
                            Some(Constraint::Length(1)).into_iter().cycle().take(4).chain(Some(Constraint::Length(2 + text.height() as u16))),
                        )
                        .split(block.inner(body_rects[1]));

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
                        // TODO: Scroll
                        frame
                            .render_widget(Paragraph::new(text).fg(right_color).block(Block::bordered().title("Payload").fg(right_color)), vrects[4]);
                    },
                    TraceJustAct::AdvanceTime { timestamp } => {
                        // Render the time
                        frame.render_widget(
                            Paragraph::new({
                                let mut text = Text::from("Time advanced to: ");
                                text.push_span(Span::from(timestamp.to_string()).bold());
                                text
                            })
                            .fg(right_color),
                            block.inner(body_rects[1]),
                        );
                    },
                    TraceJustAct::EnactAction { who, to, action } => {
                        // Prepare the layout
                        let denot: &Result<(Denotation, Text<'static>), _> = self.denot_cache.get(&action.id).unwrap_or_else(|| {
                            panic!("Failed to find action \"{} {}\" in denotation cache after list construction!", action.id.0, action.id.1)
                        });
                        let vrects = Layout::vertical(
                            [Constraint::Length(1); 13]
                                .into_iter()
                                .chain([Constraint::Length(1)].into_iter().cycle().take({
                                    let n: usize = denot.as_ref().map(|(d, _)| <Denotation as InfallibleMap<Effect>>::len(d)).unwrap_or(0);
                                    if n > 0 { n } else { 1 }
                                }))
                                .chain([Constraint::Length(denot.as_ref().map(|(_, text)| text.height() as u16).unwrap_or(0))]),
                        )
                        .split(block.inner(body_rects[1]));

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
                                        Selector::Agent(agent) => agent.as_ref(),
                                        Selector::All => "<everyone>",
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
                            Ok((denot, truths)) => {
                                // Validity
                                frame.render_widget(
                                    Paragraph::new({
                                        let mut text = Text::from("Validity : ");
                                        text.push_span(if denot.is_valid() {
                                            Span::from("OK").bold().green()
                                        } else {
                                            Span::from("INVALID").bold().red()
                                        });
                                        text
                                    })
                                    .fg(right_color),
                                    vrects[i],
                                );
                                i += 1;
                                // Effects
                                frame.render_widget(Paragraph::new("Effects  : ").fg(right_color), vrects[i]);
                                i += 1;
                                if !<Denotation as InfallibleMap<Effect>>::is_empty(&denot) {
                                    let mut effects: Vec<&Effect> = <Denotation as InfallibleMap<Effect>>::iter(&denot).collect();
                                    effects.sort_by_key(|effect| format!("{effect:?}"));
                                    for effect in effects {
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
                                frame.render_stateful_widget(
                                    ScrollArea::new(
                                        Paragraph::new(truths.clone())
                                            .block(Block::bordered().title("Justification truths").fg(right_color))
                                            .fg(right_color),
                                        (2 + truths.width() as u16, 2 + truths.height() as u16),
                                    ),
                                    vrects[i],
                                    &mut self.right_hscroll,
                                );
                            },
                            Err(err) => todo!(),
                        }
                    },
                    TraceJustAct::StateMessage { who, to, msg } => {
                        // Prepare the layout
                        let text = Text::from(msg.payload.lines().map(|l| Line::raw(l)).collect::<Vec<Line>>());
                        let vrects = Layout::vertical(
                            Some(Constraint::Length(1)).into_iter().cycle().take(6).chain(Some(Constraint::Length(2 + text.height() as u16))),
                        )
                        .split(block.inner(body_rects[1]));

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
                                        Selector::Agent(agent) => agent.as_ref(),
                                        Selector::All => "<everyone>",
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
                        // TODO: Scroll
                        frame
                            .render_widget(Paragraph::new(text).fg(right_color).block(Block::bordered().title("Payload").fg(right_color)), vrects[6]);
                    },
                },

                Trace::Dataplane(trace) => match trace {
                    TraceDataplane::Read { who, id, contents } => {
                        // Prepare the layout
                        let scontents: Option<Cow<str>> = contents.as_ref().map(Cow::as_ref).map(String::from_utf8_lossy);
                        let lines = scontents
                            .into_iter()
                            .map(|c| c.lines().map(|l| Line::raw(l.to_string())).collect::<Vec<Line>>())
                            .flatten()
                            .collect::<Vec<Line>>();
                        let lines = if !lines.is_empty() { lines } else { vec![Line::from("<no content>")] };
                        let text = Text::from(lines);
                        let vrects = Layout::vertical([Constraint::Length(1); 3].into_iter().chain([Constraint::Length(2 + text.height() as u16)]))
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

                        // Render the payload
                        if contents.is_some() {
                            frame.render_widget(
                                Paragraph::new(text).block(Block::bordered().title("Contents read").fg(right_color)).fg(right_color),
                                vrects[3],
                            );
                        }
                    },
                    TraceDataplane::Write { who, id, new, contents } => {
                        // Prepare the layout
                        let scontents: Cow<str> = String::from_utf8_lossy(contents);
                        let lines = scontents.lines().map(|l| Line::raw(l.to_string())).collect::<Vec<Line>>();
                        let lines = if !lines.is_empty() { lines } else { vec![Line::from("<no content>")] };
                        let text = Text::from(lines);
                        let vrects = Layout::vertical([Constraint::Length(1); 3].into_iter().chain([Constraint::Length(2 + text.height() as u16)]))
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

                        // Render the payload
                        frame.render_widget(
                            Paragraph::new(text).block(Block::bordered().title("Contents written").fg(right_color)).fg(right_color),
                            vrects[3],
                        );
                    },
                },
            }
        }



        // Footer
        if *self.focus == Focus::Trace {
            let hrects = Layout::horizontal([Constraint::Fill(1); 3].as_slice()).split(vrects[2]);

            render_centered_text(frame, press_to("Q", "quit"), hrects[0]);
            render_centered_text(frame, press_to("Esc", "close trace"), hrects[1]);
            render_centered_text(frame, press_or_to("Shift+←", "Tab", "switch to list"), hrects[2]);
        } else {
            let n_boxes: usize = 2 + self.traces_state.selected().map(|_| 2).unwrap_or(0) + self.traces_opened.map(|_| 1).unwrap_or(0);
            let hrects = Layout::horizontal(Some(Constraint::Fill(1)).into_iter().cycle().take(n_boxes)).split(vrects[2]);

            let mut i: usize = 0;
            render_centered_text(
                frame,
                if self.traces_state.selected().is_some() { press_or_to("Q", "Esc", "quit") } else { press_to("Q", "quit") },
                hrects[i],
            );
            i += 1;
            if self.traces_state.selected().is_some() {
                render_centered_text(frame, press_to("Esc", "unselect"), hrects[i]);
                i += 1;
            }
            if self.traces_opened.is_some() {
                render_centered_text(frame, press_or_to("Shift+→", "Tab", "switch to trace"), hrects[i]);
                i += 1;
            }
            render_centered_text(frame, press_or_to("↑", "↓", "select traces"), hrects[i]);
            i += 1;
            if self.traces_state.selected().is_some() {
                render_centered_text(frame, press_to("Enter", "view a trace"), hrects[i]);
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
                if *self.focus == Focus::List && self.traces_state.selected().is_some() {
                    // Make the currently selected one, opened
                    *self.traces_opened = self.traces_state.selected();
                    *self.focus = Focus::Trace;
                    self.right_hscroll.reset();
                    self.right_vscroll.reset();
                }
                Ok(ControlFlow::Continue(()))
            },
            Event::Key(KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Received key event UP");
                if !self.traces.is_empty() && *self.focus == Focus::List {
                    match self.traces_state.selected() {
                        Some(i) if i == 0 => self.traces_state.select(None),
                        Some(i) => self.traces_state.select(Some(i - 1)),
                        None => self.traces_state.select(Some(self.traces.len() - 1)),
                    }
                    // Also update the opened one if any
                    if self.traces_opened.is_some() {
                        *self.traces_opened = self.traces_state.selected();
                        self.right_hscroll.reset();
                        self.right_vscroll.reset();
                        if self.traces_opened.is_none() {
                            *self.focus = Focus::List;
                        }
                    }
                } else if *self.focus == Focus::Trace {
                    self.right_vscroll.scroll_up();
                }
                Ok(ControlFlow::Continue(()))
            },
            Event::Key(KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Received key event DOWN");
                if !self.traces.is_empty() && *self.focus == Focus::List {
                    match self.traces_state.selected() {
                        Some(i) if i >= self.traces.len() - 1 => self.traces_state.select(None),
                        Some(i) => self.traces_state.select(Some(i + 1)),
                        None => self.traces_state.select(Some(0)),
                    }
                    // Also update the opened one if any
                    if self.traces_opened.is_some() {
                        *self.traces_opened = self.traces_state.selected();
                        self.right_hscroll.reset();
                        self.right_vscroll.reset();
                        if self.traces_opened.is_none() {
                            *self.focus = Focus::List;
                        }
                    }
                } else if *self.focus == Focus::Trace {
                    self.right_vscroll.scroll_down();
                }
                Ok(ControlFlow::Continue(()))
            },
            Event::Key(KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                if *self.focus == Focus::Trace {
                    self.right_hscroll.scroll_left();
                }
                Ok(ControlFlow::Continue(()))
            },
            Event::Key(KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                if *self.focus == Focus::Trace {
                    self.right_hscroll.scroll_right();
                }
                Ok(ControlFlow::Continue(()))
            },
            Event::Key(KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ }) => {
                debug!(target: "Main", "Received key event ESC");
                if *self.focus == Focus::List {
                    if self.traces_state.selected().is_some() {
                        self.traces_state.select(None);
                        *self.traces_opened = None;
                        *self.focus = Focus::List;
                        Ok(ControlFlow::Continue(()))
                    } else {
                        debug!(target: "Main", "Quitting...");
                        Ok(ControlFlow::Break(()))
                    }
                } else {
                    *self.traces_opened = None;
                    *self.focus = Focus::List;
                    Ok(ControlFlow::Continue(()))
                }
            },

            // Focus management
            Event::Key(KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::SHIFT, kind: KeyEventKind::Press, state: _ })
            | Event::Key(KeyEvent { code: KeyCode::Tab, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ })
                if *self.focus == Focus::List =>
            {
                // If it's opened, we can shift
                if self.traces_opened.is_some() {
                    *self.focus = Focus::Trace;
                }
                Ok(ControlFlow::Continue(()))
            },
            Event::Key(KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::SHIFT, kind: KeyEventKind::Press, state: _ })
            | Event::Key(KeyEvent { code: KeyCode::Tab, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: _ })
                if *self.focus == Focus::Trace =>
            {
                // If it's opened, we can shift
                if self.traces_opened.is_some() {
                    *self.focus = Focus::List;
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
        errors: Arc<Mutex<VecDeque<Error>>>,
        output: Arc<Mutex<Vec<Trace<'static>>>>,
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
