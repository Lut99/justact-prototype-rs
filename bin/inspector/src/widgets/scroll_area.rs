//  SCROLL AREA.rs
//    by Lut99
//
//  Created:
//    21 Jan 2025, 20:26:24
//  Last edited:
//    22 Jan 2025, 17:38:39
//  Auto updated?
//    Yes
//
//  Description:
//!   Implements a widget that can scroll its content.
//

#![allow(unused)]

use std::cmp::min;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, StatefulWidget, Widget};


/***** HELPER FUNCTIONS *****/
/// Does the math-y part of the scrolling.
///
/// # Arguments
/// - `scroll`: The amount of scrolling to apply.
/// - `outer`: The size of the _outer_ area (i.e., visible area).
/// - `inner`: The size of the _inner_ area (i.e., total area).
/// - `inner_buf`: The rendered inside area, part of which to copy to the `outer_buf`.
/// - `outer_buf`: The outside area to copy a smaller part of the `inner_buf` to.
fn scroll(scroll: (u16, u16), outer: Rect, inner: Rect, inner_buf: &Buffer, outer_buf: &mut Buffer) {
    // Next, decide which part of the inner window to copy
    let cut: Rect = Rect::new(
        scroll.0,
        scroll.1,
        outer.width - if scroll.0 + outer.width > inner.width { (scroll.0 + outer.width) - inner.width } else { 0 },
        outer.height - if scroll.1 + outer.height > inner.height { (scroll.1 + outer.height) - inner.height } else { 0 },
    );

    // Then we copy that part into the output buffer (with the appropriate offsets)
    log::debug!("Cutting {}x{} at {},{}", cut.width, cut.height, cut.x, cut.y);
    for y in 0..cut.height {
        let inner_y: u16 = cut.y + y;
        let outer_y: u16 = outer.y + y;
        for x in 0..cut.width {
            let inner_x: u16 = cut.x + x;
            let outer_x: u16 = outer.x + x;
            let outer_width: u16 = outer_buf.area.width;
            outer_buf.content[(outer_y * outer_width + outer_x) as usize] = inner_buf.content[(inner_y * inner.width + inner_x) as usize].clone();
        }
    }
}





/***** AUXILLARY *****/
/// Something [`Frame`](ratatui::Frame)-like but not really.
#[derive(Debug)]
pub struct ScrollFrame<'a> {
    /// The buffer we're referencing.
    buffer: &'a mut Buffer,
    /// The render area.
    area:   Rect,
}
impl<'a> ScrollFrame<'a> {
    /// Returns the area of the current frame.
    ///
    /// Like [`Frame::area()`](ratatui::Frame::area()), this value is guaranteed not to change.
    ///
    /// # Returns
    /// A [`Rect`] representing the rendered area.
    #[inline]
    pub const fn area(&self) -> Rect { self.area }

    /// Renders a particular [`Widget`] to the backend buffer.
    ///
    /// # Arguments
    /// - `widget`: The [`Widget`] to render.
    /// - `area`: Some [`Rect`] describing to what area of the buffer to render to.
    #[inline]
    pub fn render_widget(&mut self, widget: impl Widget, area: Rect) { widget.render(area, self.buffer) }

    /// Renders a particular [`StatefulWidget`] to the backend buffer.
    ///
    /// # Arguments
    /// - `widget`: The [`StatefulWidget`] to render.
    /// - `area`: Some [`Rect`] describing to what area of the buffer to render to.
    /// - `state`: The `widget`'s state to update while rendering.
    #[inline]
    pub fn render_stateful_widget<W: StatefulWidget>(&mut self, widget: W, area: Rect, state: &mut W::State) {
        widget.render(area, self.buffer, state)
    }
}



/// The state that is adapted such that the [`ScrollArea`] scrolls.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ScrollState {
    /// The coordinates that offset the scroll area (as an x x y pair).
    pos:   (u16, u16),
    /// A buffer for caching purposes.
    cache: Buffer,
}

// Constructors
impl Default for ScrollState {
    #[inline]
    fn default() -> Self { Self::new() }
}
impl ScrollState {
    /// Constructs a new ScrollState.
    ///
    /// # Returns
    /// A new ScrollState ready for keeping track of scroll states.
    #[inline]
    pub fn new() -> Self { Self { pos: (0, 0), cache: Buffer::empty(Rect::ZERO) } }
}

// Scrolling
impl ScrollState {
    /// Scrolls the scroll area to the start (topleft-most).
    ///
    /// # Returns
    /// A mutable reference to Self for chaining.
    #[inline]
    pub const fn reset(&mut self) -> &mut Self {
        self.pos = (0, 0);
        self
    }

    /// Scrolls the scroll area one line up.
    ///
    /// # Returns
    /// A mutable reference to Self for chaining.
    #[inline]
    pub const fn scroll_up(&mut self) -> &mut Self { self.scroll_up_by(1) }
    /// Scrolls the scroll area up.
    ///
    /// It will automatically clip the scrolling.
    ///
    /// # Arguments
    /// - `n`: The number of lines to scroll up.
    ///
    /// # Returns
    /// A mutable reference to Self for chaining.
    #[inline]
    pub const fn scroll_up_by(&mut self, n: u16) -> &mut Self {
        self.pos.1 = self.pos.1.saturating_sub(n);
        self
    }

    /// Scrolls the scroll area one character right.
    ///
    /// # Returns
    /// A mutable reference to Self for chaining.
    #[inline]
    pub const fn scroll_right(&mut self) -> &mut Self { self.scroll_right_by(1) }
    /// Scrolls the scroll area right.
    ///
    /// It will automatically clip the scrolling.
    ///
    /// # Arguments
    /// - `n`: The number of character to scroll right.
    ///
    /// # Returns
    /// A mutable reference to Self for chaining.
    #[inline]
    pub const fn scroll_right_by(&mut self, n: u16) -> &mut Self {
        self.pos.0 = self.pos.0.saturating_add(n);
        self
    }

    /// Scrolls the scroll area one line down.
    ///
    /// # Returns
    /// A mutable reference to Self for chaining.
    #[inline]
    pub const fn scroll_down(&mut self) -> &mut Self { self.scroll_down_by(1) }
    /// Scrolls the scroll area down.
    ///
    /// It will automatically clip the scrolling.
    ///
    /// # Arguments
    /// - `n`: The number of lines to scroll down.
    ///
    /// # Returns
    /// A mutable reference to Self for chaining.
    #[inline]
    pub const fn scroll_down_by(&mut self, n: u16) -> &mut Self {
        self.pos.1 = self.pos.1.saturating_add(n);
        self
    }

    /// Scrolls the scroll area one character left.
    ///
    /// # Returns
    /// A mutable reference to Self for chaining.
    #[inline]
    pub const fn scroll_left(&mut self) -> &mut Self { self.scroll_left_by(1) }
    /// Scrolls the scroll area left.
    ///
    /// It will automatically clip the scrolling.
    ///
    /// # Arguments
    /// - `n`: The number of character to scroll left.
    ///
    /// # Returns
    /// A mutable reference to Self for chaining.
    #[inline]
    pub const fn scroll_left_by(&mut self, n: u16) -> &mut Self {
        self.pos.0 = self.pos.0.saturating_sub(n);
        self
    }
}





/***** LIBRARY *****/
/// The ScrollArea will render a stateful widget to a larger area, and then cut that area
/// to a smaller one.
///
/// This smaller area can then be scrolled using the [`ScrollState`].
///
/// See the [`ScrollArea`] for non-stateful widgets.
#[derive(Debug, Clone)]
pub struct ScrollArea<'a, F> {
    /// A block to render around this area's contents.
    block: Option<Block<'a>>,
    /// The scrolled area, e.g., the size of the thing we're rendering (as a width x height pair).
    inner: Rect,
    /// The closure doing the rendering.
    render_callback: F,
}
impl<'a> ScrollArea<'a, ()> {
    /// Constructs a new ScrollArea.
    ///
    /// # Arguments
    /// - `inner`: The size of the scroll area's inner area (i.e., the size of the area the inner
    ///   widget renders to). Given as `(width x height)`.
    ///
    /// # Returns
    /// A new ScrollArea that can be rendered.
    #[inline]
    pub const fn new(inner: Rect) -> Self { Self { block: None, inner, render_callback: () } }
}
impl<'a, F> ScrollArea<'a, F> {
    /// Exposes the internal buffer for rendering the inner area.
    ///
    /// The inner area will be sized as given when creating the ScrollArea. Note that whatever was
    /// there first will be cleared upon a new call of this function.
    ///
    /// Also note that the rendering won't take place _immediately_, but rather when this
    /// ScrollArea itself is being rendered.
    ///
    /// # Arguments
    /// - `render_callback`: Some [`FnOnce`]-closure that takes something very much looking like a
    ///   [`Frame`](ratatui::Frame) (but not really because we can't construct it) and allows you
    ///   to render to the ScrollArea's inner area.
    ///
    /// # Returns
    /// An identical ScrollArea that will render its inner contents according to `render_callback`.
    #[inline]
    pub fn render_inner<F2: for<'f> FnOnce(ScrollFrame<'f>)>(self, render_callback: F2) -> ScrollArea<'a, F2> {
        ScrollArea { block: self.block, inner: self.inner, render_callback }
    }

    /// Adds a block around this area.
    ///
    /// # Arguments
    /// - `block`: The block to add.
    ///
    /// # Returns
    /// `Self` for chaining.
    #[inline]
    pub fn block<'b>(mut self, block: Block<'b>) -> ScrollArea<'b, F> {
        ScrollArea { block: Some(block), inner: self.inner, render_callback: self.render_callback }
    }
}
impl<'a, F: for<'f> FnOnce(ScrollFrame<'f>)> StatefulWidget for ScrollArea<'a, F> {
    type State = ScrollState;

    #[inline]
    fn render(self, outer: Rect, outer_buf: &mut Buffer, state: &mut Self::State) {
        // Re-compute the outer if blocking
        let outer = if let Some(block) = self.block {
            // We can already render the block, why not
            let inner_outer: Rect = block.inner(outer);
            block.render(outer, outer_buf);

            // Compute the rendering area for us
            inner_outer
        } else {
            outer
        };

        // Render the given widget to a buffer the size of the inner area first.
        let inner: Rect = Rect::new(0, 0, self.inner.width, self.inner.height);
        state.cache.resize(inner);
        state.cache.reset();
        (self.render_callback)(ScrollFrame { buffer: &mut state.cache, area: inner });

        // Now bound the scroll state to not go beyond the inner frame
        if state.pos.0 + outer.width > self.inner.width {
            state.pos.0 = state.pos.0.saturating_sub((state.pos.0 + outer.width) - self.inner.width);
        }
        if state.pos.1 + outer.height > self.inner.height {
            state.pos.1 = state.pos.1.saturating_sub((state.pos.1 + outer.height) - self.inner.height);
        }

        // Run the math
        scroll(state.pos, outer, inner, &state.cache, outer_buf);
    }
}
