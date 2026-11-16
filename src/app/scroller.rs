//! Active window title marquee scroller.

use crate::theme::MAX_WINDOW_CHARS;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowScroller {
    pub raw_title: String,
    pub raw_class: String,
    pub offset: usize,
    pub pause_ticks: u8,
}

impl WindowScroller {
    pub fn set_window(&mut self, class: String, title: String) {
        let normalized = if !title.trim().is_empty() {
            title
        } else if !class.trim().is_empty() {
            class.clone()
        } else {
            String::new()
        };
        if self.raw_title != normalized || self.raw_class != class {
            self.raw_title = normalized;
            self.raw_class = class;
            self.offset = 0;
            self.pause_ticks = 4;
        }
    }

    #[must_use]
    pub fn needs_scroll(&self) -> bool {
        self.raw_title.chars().count() > MAX_WINDOW_CHARS
    }

    pub fn advance(&mut self) {
        if !self.needs_scroll() {
            self.offset = 0;
            return;
        }
        if self.pause_ticks > 0 {
            self.pause_ticks -= 1;
            return;
        }
        let loop_len = self.raw_title.chars().count() + 5;
        self.offset = (self.offset + 1) % loop_len;
        if self.offset == 0 {
            self.pause_ticks = 6;
        }
    }

    #[must_use]
    pub fn visible_text(&self) -> String {
        let len = self.raw_title.chars().count();
        if len <= MAX_WINDOW_CHARS {
            return self.raw_title.clone();
        }
        let looped: Vec<char> = format!("{}  \u{2022}  ", self.raw_title).chars().collect();
        let mut out = String::with_capacity(MAX_WINDOW_CHARS * 2);
        for i in 0..MAX_WINDOW_CHARS {
            out.push(looped[(self.offset + i) % looped.len()]);
        }
        out
    }
}
