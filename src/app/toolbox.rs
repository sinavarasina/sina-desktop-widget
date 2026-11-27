//! Toolbox tool catalog exposed inside Quick Menu's expandable Toolbox strip and IPC.

use waywi::prelude::*;

use crate::theme::{BLUE, MAUVE, PEACH};

/// Desktop utility tools available inside the Quick Menu Toolbox strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolboxTool {
    Annotate,
    Zoom,
    ColorPicker,
    Screenshot,
}

impl ToolboxTool {
    pub const ALL: [Self; 4] = [
        Self::Annotate,
        Self::Zoom,
        Self::ColorPicker,
        Self::Screenshot,
    ];

    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            // Every glyph below is verified present in MesloLGSNerdFont-Regular.ttf.
            Self::Annotate => "\u{f040}",
            Self::Zoom => "\u{f00e}",
            Self::ColorPicker => "\u{f1fb}",
            Self::Screenshot => "\u{f030}",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Annotate => "Annotate",
            Self::Zoom => "Magnifier",
            Self::ColorPicker => "Color Picker",
            Self::Screenshot => "Screenshot",
        }
    }

    #[must_use]
    pub const fn accent(self) -> Color {
        match self {
            Self::Annotate | Self::Zoom => MAUVE,
            Self::ColorPicker => PEACH,
            Self::Screenshot => BLUE,
        }
    }
}
