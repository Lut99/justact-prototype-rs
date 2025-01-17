//  SCROLL AREA.rs
//    by Lut99
//
//  Created:
//    17 Jan 2025, 11:12:20
//  Last edited:
//    17 Jan 2025, 11:53:55
//  Auto updated?
//    Yes
//
//  Description:
//!   Defines a widget that scrolls in any direction.
//

use std::ops::{Deref, DerefMut};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{StatefulWidget, Widget};


/***** AUXILLARY *****/
/// The external state for the [`ScrollArea`].
///
/// Keeps track of where we at, scrolling.
#[derive(Clone, Copy, Debug)]
pub struct ScrollState {
    // The current coordinates of the top-left window of the area.
    pos: (i32, i32),
}

// Constructors
impl Default for ScrollState {
    #[inline]
    fn default() -> Self { Self::new() }
}
impl ScrollState {
    /// Constructor for the ScrollState.
    ///
    /// # Returns
    /// A new ScrollState that is not yet scrolled.
    #[inline]
    pub const fn new() -> Self { Self { pos: (0, 0) } }
}

// Scroll functions
impl ScrollState {
    /// Scrolls left by 1 character.
    #[allow(unused)]
    #[inline]
    pub fn scroll_left(&mut self) -> &mut Self { self.scroll_left_by(1) }
    /// Scrolls to the left by `n` steps.
    ///
    /// # Arguments
    /// - `n`: The number of characters to scroll left.
    #[allow(unused)]
    pub fn scroll_left_by(&mut self, n: u16) -> &mut Self {
        if self.pos.0 > -(u16::MAX as i32) {
            self.pos.0 -= 1;
        }
        self
    }

    /// Scrolls right by 1 character.
    #[allow(unused)]
    #[inline]
    pub fn scroll_right(&mut self) -> &mut Self { self.scroll_right_by(1) }
    /// Scrolls to the right by `n` steps.
    ///
    /// # Arguments
    /// - `n`: The number of characters to scroll right.
    #[allow(unused)]
    pub fn scroll_right_by(&mut self, n: u16) -> &mut Self {
        if self.pos.0 < (u16::MAX as i32) {
            self.pos.0 += 1;
        }
        self
    }

    /// Scrolls up by 1 line.
    #[allow(unused)]
    #[inline]
    pub fn scroll_up(&mut self) -> &mut Self { self.scroll_up_by(1) }
    /// Scrolls up by `n` steps.
    ///
    /// # Arguments
    /// - `n`: The number of lines to scroll up.
    #[allow(unused)]
    pub fn scroll_up_by(&mut self, n: u16) -> &mut Self {
        if self.pos.1 > -(u16::MAX as i32) {
            self.pos.1 -= 1;
        }
        self
    }

    /// Scrolls down by 1 line.
    #[allow(unused)]
    #[inline]
    pub fn scroll_down(&mut self) -> &mut Self { self.scroll_down_by(1) }
    /// Scrolls down by `n` steps.
    ///
    /// # Arguments
    /// - `n`: The number of lines to scroll down.
    #[allow(unused)]
    pub fn scroll_down_by(&mut self, n: u16) -> &mut Self {
        if self.pos.1 < (u16::MAX as i32) {
            self.pos.1 += 1;
        }
        self
    }
}



/// Defines an interface that extends [`Widget`]s to make them scrollable.
pub trait Scrollable: Sized + Widget {
    /// Make this widget scrollable by rendering it within a [`ScrollArea`].
    ///
    /// # Arguments
    /// - `inner_area`: The inner area to which the nested widget will render. Go bananas, this is
    ///   what is scrolled over. Note it's given as a (width, height)-pair.
    /// - `scroll_state`: A [`ScrollState`] that describes which scroll to apply.
    ///
    /// # Returns
    /// A [`ScrollArea`] that wraps self such that it is rendered correctly.
    #[allow(unused)]
    fn scroll(self, inner_area: (u16, u16), scroll_state: &ScrollState) -> ScrollArea<Self>;
}
impl<T: Sized + Widget> Scrollable for T {
    #[inline]
    fn scroll(self, inner_area: (u16, u16), scroll_state: &ScrollState) -> ScrollArea<Self> {
        ScrollArea { widget: self, inner_area: Rect::new(0, 0, inner_area.0, inner_area.1), state: scroll_state }
    }
}

/// Defines an interface that extends [`StatefulWidget`]s to make them scrollable.
pub trait StatefulScrollable: Sized + StatefulWidget {
    /// Make this (stateful) widget scrollable by rendering it within a [`ScrollArea`].
    ///
    /// # Arguments
    /// - `inner_area`: The inner area to which the nested widget will render. Go bananas, this is
    ///   what is scrolled over. Note it's given as a (width, height)-pair.
    /// - `scroll_state`: A [`ScrollState`] that describes which scroll to apply.
    ///
    /// # Returns
    /// A [`ScrollArea`] that wraps self such that it is rendered correctly.
    #[allow(unused)]
    fn scroll(self, inner_area: (u16, u16), scroll_state: &ScrollState) -> ScrollArea<Self>;
}
impl<T: Sized + StatefulWidget> StatefulScrollable for T {
    #[inline]
    fn scroll(self, inner_area: (u16, u16), scroll_state: &ScrollState) -> ScrollArea<Self> {
        ScrollArea { widget: self, inner_area: Rect::new(0, 0, inner_area.0, inner_area.1), state: scroll_state }
    }
}





/***** LIBRARY *****/
/// Implements an area that can scroll in any direction.
///
/// This is useful to display larger areas of text, for example.
#[derive(Clone, Debug)]
pub struct ScrollArea<'s, W> {
    // The widget to scroll when rendering
    widget:     W,
    /// The inner area to actually scroll.
    inner_area: Rect,
    /// The area describing how to scroll.
    state:      &'s ScrollState,
}

// Deref
impl<'s, W> Deref for ScrollArea<'s, W> {
    type Target = W;

    #[inline]
    fn deref(&self) -> &Self::Target { &self.widget }
}
impl<'s, W> DerefMut for ScrollArea<'s, W> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.widget }
}

// Rendering
impl<'s, W> StatefulWidget for ScrollArea<'s, W>
where
    W: StatefulWidget,
{
    type State = W::State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        // Render the content first
        let mut inner_buf = Buffer::empty(self.inner_area);
        self.widget.render(self.inner_area, &mut inner_buf, state);

        // Decide a rectangle to cut
        let pos: (i32, i32) = self.state.pos;
        let scroll_area: Rect = Rect::new(pos.0.clamp(0, u16::MAX as i32) as u16, pos.1.clamp(0, u16::MAX as i32) as u16, area.width, area.height);
        let inner_area: Rect = Rect::new(
            (-pos.0.clamp(-(u16::MAX as i32), 0)) as u16,
            (-pos.1.clamp(-(u16::MAX as i32), 0)) as u16,
            self.inner_area.width,
            self.inner_area.height,
        );
        let source: Rect = inner_area.intersection(scroll_area).clamp(self.inner_area);

        // This part, we copy the target lines from the buffer
        let target: Rect = Rect::new(area.x, area.y, source.width, source.height);
        for line in 0..target.height {
            let left: usize = ((target.y + line) * buf.area.width + target.x) as usize;
            let inner_left: usize = ((source.y + line) * inner_buf.area.width + source.x) as usize;
            for cell in 0..target.width {
                buf.content[left + cell as usize] = inner_buf.content[inner_left + cell as usize].clone();
            }
        }
    }
}
