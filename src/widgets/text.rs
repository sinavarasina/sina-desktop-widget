//! Shared text formatting helpers used by both bar modules and popup widgets.

use waywi::prelude::MprisSnapshot;

/// Formats "artist - album" (or whichever part exists, falling back to the player identity).
pub fn format_artist_album(p: &MprisSnapshot) -> String {
    let artist = p.artist.as_deref().unwrap_or("").trim();
    let album = p.album.as_deref().unwrap_or("").trim();
    match (!artist.is_empty(), !album.is_empty()) {
        (true, true) => format!("{artist} - {album}"),
        (true, false) => artist.to_owned(),
        (false, true) => album.to_owned(),
        (false, false) => p
            .identity
            .as_deref()
            .unwrap_or("Media")
            .trim()
            .to_owned(),
    }
}

/// Visual-column aware marquee slicer (CJK / fullwidth chars count as ~1.5x ASCII width).
pub fn marquee_slice(input: &str, max_cols: usize, tick: u32) -> String {
    let clean = input.trim();
    let budget_half_cols = max_cols * 2;
    let total_half_cols: usize = clean.chars().map(char_half_cols).sum();
    if total_half_cols <= budget_half_cols {
        return clean.to_owned();
    }
    let looped: Vec<char> = format!("{clean}   \u{2022}   ").chars().collect();
    let pause = 5usize;
    let cycle = looped.len() + pause;
    let step = (tick as usize) % cycle;
    let offset = step.saturating_sub(pause) % looped.len();
    let mut out = String::with_capacity(max_cols * 2);
    let mut used = 0usize;
    let mut idx = 0usize;
    while used < budget_half_cols {
        let ch = looped[(offset + idx) % looped.len()];
        let w = char_half_cols(ch);
        if used + w > budget_half_cols {
            break;
        }
        out.push(ch);
        used += w;
        idx += 1;
    }
    out
}

#[must_use]
const fn char_half_cols(c: char) -> usize {
    let u = c as u32;
    if (u >= 0x1100 && u <= 0x115F)
        || (u >= 0x2E80 && u <= 0xA4CF)
        || (u >= 0xAC00 && u <= 0xD7A3)
        || (u >= 0xF900 && u <= 0xFAFF)
        || (u >= 0xFE10 && u <= 0xFE6F)
        || (u >= 0xFF00 && u <= 0xFF60)
        || (u >= 0x1F300 && u <= 0x1FAFF)
    {
        3
    } else {
        2
    }
}
