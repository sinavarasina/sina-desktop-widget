//! Quick status popup, clock menu toggle, and cancelable logind power action handlers.

use waywi::prelude::*;

use crate::app::handlers::ipc::close_other_overlays;
use crate::app::quick_menu::PowerAction;
use crate::app::state::{SoundCue, State};

pub fn handle_logind_event(state: &mut State, ev: LogindEvent) {
    if let LogindEvent::Snapshot(snap) = ev {
        state.quick_menu.logind = snap;
    }
}

pub fn handle_clock_menu_click(state: &mut State, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    if state.quick_menu.is_popup_open() {
        state.quick_menu.close_popup();
        state.settings_ui.unfocus_inputs();
        state.settings_ui.open_dropdown = None;
        if !state.hovered.is_clock_group() {
            state.clock_menu_anim.set_open(false);
        }
        state.play_sound(SoundCue::MenuPopdown, ctx);
    } else {
        close_other_overlays(state, Some("quick_menu"));
        refresh_power_capabilities_if_empty(state, ctx);
        state.quick_menu.open_popup(ctx);
        state.clock_menu_anim.set_open(true);
        state.clock_menu_leave_deadline = None;
        state.play_sound(SoundCue::MenuPopup, ctx);
    }
}

pub fn handle_quick_menu_toggle_power(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    refresh_power_capabilities_if_empty(state, ctx);
    state.quick_menu.toggle_power_menu();
    state.settings_ui.unfocus_inputs();
    state.settings_ui.open_dropdown = None;
    let cue = if state.quick_menu.power_menu_open {
        SoundCue::ToggleOn
    } else {
        SoundCue::ToggleOff
    };
    state.play_sound(cue, ctx);
}

/// Schedules any power action behind the configured cancelable countdown (`0s` executes immediately).
pub fn handle_power_action(state: &mut State, action: PowerAction, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    let support = action.support(&state.quick_menu.logind.power_capabilities);
    if !support.is_supported() {
        state.quick_menu.power_error = Some(format!("{} not supported", action.label()));
        state.play_sound(SoundCue::DialogError, ctx);
        return;
    }
    let countdown = state.config.general.power_countdown_secs;
    if countdown == 0 {
        execute_power_action(state, action, ctx);
    } else {
        state.quick_menu.start_power_countdown_secs(action, countdown);
        state.play_sound(SoundCue::ToggleOn, ctx);
    }
}

/// Advances the 1-second power countdown and executes the logind action once it reaches 0.
pub fn handle_power_countdown_tick(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    let Some(mut pending) = state.quick_menu.pending_power else {
        return;
    };
    if pending.seconds_left > 1 {
        pending.seconds_left -= 1;
        state.quick_menu.pending_power = Some(pending);
        return;
    }
    state.quick_menu.pending_power = None;
    execute_power_action(state, pending.action, ctx);
}

fn power_lifecycle_cue(action: PowerAction) -> SoundCue {
    match action {
        PowerAction::Logout => SoundCue::DesktopLogout,
        PowerAction::Suspend
        | PowerAction::Hibernate
        | PowerAction::HybridSleep
        | PowerAction::SuspendThenHibernate => SoundCue::SuspendStart,
        PowerAction::PowerOff
        | PowerAction::Reboot
        | PowerAction::RebootToFirmware
        | PowerAction::SoftReboot
        | PowerAction::Halt => SoundCue::SystemShutdown,
    }
}

fn execute_power_action(state: &mut State, action: PowerAction, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    let support = action.support(&state.quick_menu.logind.power_capabilities);
    if !support.is_supported() {
        state.quick_menu.power_error = Some(format!("{} not supported", action.label()));
        state.play_sound(SoundCue::DialogError, ctx);
        return;
    }
    let interactive = support.needs_auth();
    state.quick_menu.power_error = None;

    let Ok(dbus) = ctx.dbus() else {
        return;
    };

    let res = match action {
        PowerAction::PowerOff => dbus.logind().power(waywi::dbus::PowerAction::PowerOff, interactive),
        PowerAction::Reboot => dbus.logind().power(waywi::dbus::PowerAction::Reboot, interactive),
        PowerAction::RebootToFirmware => dbus.logind().power(waywi::dbus::PowerAction::RebootToFirmwareSetup, interactive),
        PowerAction::SoftReboot => dbus.logind().power(waywi::dbus::PowerAction::SoftReboot, interactive),
        PowerAction::Suspend => dbus.logind().power(waywi::dbus::PowerAction::Suspend, interactive),
        PowerAction::Hibernate => dbus.logind().power(waywi::dbus::PowerAction::Hibernate, interactive),
        PowerAction::HybridSleep => dbus.logind().power(waywi::dbus::PowerAction::HybridSleep, interactive),
        PowerAction::SuspendThenHibernate => dbus.logind().power(waywi::dbus::PowerAction::SuspendThenHibernate, interactive),
        PowerAction::Halt => dbus.logind().power(waywi::dbus::PowerAction::Halt, interactive),
        PowerAction::Logout => dbus.logind().terminate_session(None),
    };

    if let Err(err) = res {
        state.quick_menu.power_error = Some(err.to_string());
        state.play_sound(SoundCue::DialogError, ctx);
    } else {
        state.play_sound(power_lifecycle_cue(action), ctx);
        state.quick_menu.close_popup();
        if !state.hovered.is_clock_group() {
            state.clock_menu_anim.set_open(false);
        }
    }
}

pub fn refresh_power_capabilities_if_empty(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    let caps = &state.quick_menu.logind.power_capabilities;
    let empty = !caps.power_off.is_supported()
        && !caps.reboot.is_supported()
        && !caps.suspend.is_supported()
        && !caps.hibernate.is_supported();
    if empty
        && let Ok(dbus) = ctx.dbus()
        && let Ok(queried) = dbus.logind().power_capabilities_blocking()
    {
        state.quick_menu.logind.power_capabilities = queried;
    }
}
