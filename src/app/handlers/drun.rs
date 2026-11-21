//! Native `drun` application launcher input, IME, navigation, and launch handlers.

use waywi::prelude::*;

use crate::app::handlers::events::scroll_direction_y;
use crate::app::handlers::ipc::close_other_overlays;
use crate::app::state::State;

pub fn handle_toggle_drun(state: &mut State, ctx: &mut EffectCtx<'_>) {
    if state.drun.target {
        state.drun.close();
    } else {
        close_other_overlays(state, Some("drun"));
        state.drun.open(ctx);
        resolve_visible_drun_icons(state, ctx);
    }
}

pub fn handle_drun_key(state: &mut State, ev: KeyEvent, ctx: &mut EffectCtx<'_>) {
    match ev.key {
        Key::Down | Key::Tab => {
            let delta = if ev.key == Key::Tab && ev.modifiers.contains(Modifiers::SHIFT) {
                -1
            } else {
                1
            };
            state.drun.move_selection(delta);
            resolve_visible_drun_icons(state, ctx);
            return;
        }
        Key::Up => {
            state.drun.move_selection(-1);
            resolve_visible_drun_icons(state, ctx);
            return;
        }
        Key::Right
            if state.drun.input.is_empty()
                || state.drun.input.cursor_bytes() == state.drun.input.text().len() =>
        {
            state.drun.move_selection(1);
            resolve_visible_drun_icons(state, ctx);
            return;
        }
        Key::Left if state.drun.input.is_empty() || state.drun.input.cursor_bytes() == 0 => {
            state.drun.move_selection(-1);
            resolve_visible_drun_icons(state, ctx);
            return;
        }
        _ => {}
    }

    let outcome = state.drun.input.handle_key(&ev);
    match outcome {
        TextInputOutcome::Cancelled => state.drun.close(),
        TextInputOutcome::Submitted => launch_selected_app(state),
        TextInputOutcome::Edited => {
            state.drun.refresh_matches();
            state.drun.remeasure_input(ctx);
            resolve_visible_drun_icons(state, ctx);
        }
        TextInputOutcome::CursorMoved => {
            state.drun.remeasure_input(ctx);
        }
        TextInputOutcome::Unchanged => {}
    }
}

pub fn handle_drun_ime(state: &mut State, ev: ImeEvent, ctx: &mut EffectCtx<'_>) {
    let outcome = state.drun.input.handle_ime(&ev);
    if outcome.text_changed() {
        state.drun.refresh_matches();
        resolve_visible_drun_icons(state, ctx);
    }
    if outcome.needs_redraw() {
        state.drun.remeasure_input(ctx);
    }
}

pub fn handle_drun_launch_index(state: &mut State, match_pos: usize) {
    let Some(&entry_idx) = state.drun.match_indices.get(match_pos) else {
        return;
    };
    let Some(entry) = state.drun.entries.get(entry_idx).cloned() else {
        return;
    };
    *state
        .drun
        .launch_counts
        .entry(entry.id.clone())
        .or_insert(0) += 1;
    let _ = entry.launch_detached();
    state.drun.close();
}

pub fn handle_drun_scroll(state: &mut State, sc: ScrollEvent, ctx: &mut EffectCtx<'_>) {
    let step = scroll_direction_y(sc);
    if step != 0 {
        state.drun.move_selection(step as isize);
        resolve_visible_drun_icons(state, ctx);
    }
}

fn launch_selected_app(state: &mut State) {
    if let Some(entry) = state.drun.selected_entry().cloned() {
        *state
            .drun
            .launch_counts
            .entry(entry.id.clone())
            .or_insert(0) += 1;
        let _ = entry.launch_detached();
        state.drun.close();
    }
}

pub fn resolve_visible_drun_icons(state: &mut State, ctx: &mut EffectCtx<'_>) {
    let visible: Vec<usize> = state.drun.visible_match_indices().to_vec();
    for entry_idx in visible {
        if let Some(entry) = state.drun.entries.get(entry_idx) {
            let _ = crate::app::drun::DrunState::resolve_entry_icon(
                entry,
                &mut state.icon_lookup,
                &mut state.path_asset_cache,
                ctx,
            );
        }
    }
}
