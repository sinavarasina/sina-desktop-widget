//! Bluetooth management popup state and BlueZ pairing agent state.

use waywi::prelude::*;

const BT_POPUP_ANIM_STEP: f32 = 16.0 / 180.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPairingPrompt {
    pub request_id: u64,
    pub device_path: String,
    pub label: String,
    pub needs_pin: bool,
    pub needs_passkey: bool,
}

#[derive(Default)]
pub struct BluetoothUiState {
    pub popup_target: bool,
    pub popup_progress: f32,
    pub popup_left_x: f32,
    pub scroll_index: usize,
    pub confirm_remove_path: Option<String>,
    pub pending_prompt: Option<PendingPairingPrompt>,
}

impl BluetoothUiState {
    #[must_use]
    pub fn is_popup_visible(&self) -> bool {
        self.popup_target || self.popup_progress > 0.001
    }

    #[must_use]
    pub fn is_animating_popup(&self) -> bool {
        let target = if self.popup_target { 1.0 } else { 0.0 };
        (self.popup_progress - target).abs() > 0.001
    }

    pub fn step_popup_animation(&mut self) {
        let target = if self.popup_target { 1.0_f32 } else { 0.0_f32 };
        if (self.popup_progress - target).abs() <= BT_POPUP_ANIM_STEP {
            self.popup_progress = target;
        } else if self.popup_progress < target {
            self.popup_progress = (self.popup_progress + BT_POPUP_ANIM_STEP).min(1.0);
        } else {
            self.popup_progress = (self.popup_progress - BT_POPUP_ANIM_STEP).max(0.0);
        }
    }

    pub fn toggle_popup(&mut self) {
        self.popup_target = !self.popup_target;
        if !self.popup_target {
            self.confirm_remove_path = None;
        }
    }

    pub fn close_popup(&mut self) {
        self.popup_target = false;
        self.confirm_remove_path = None;
    }

    pub fn apply_agent_event(&mut self, ev: BluetoothAgentEvent, snap: &BluetoothSnapshot) -> Option<PendingPairingPrompt> {
        match ev {
            BluetoothAgentEvent::Request(req) => {
                let prompt = match req {
                    BluetoothAgentRequest::RequestConfirmation {
                        request_id,
                        device_path,
                        passkey,
                    } => PendingPairingPrompt {
                        request_id,
                        label: format!(
                            "Confirm {:06} for {}",
                            passkey,
                            device_label(snap, &device_path)
                        ),
                        device_path,
                        needs_pin: false,
                        needs_passkey: false,
                    },
                    BluetoothAgentRequest::RequestAuthorization {
                        request_id,
                        device_path,
                    } => PendingPairingPrompt {
                        request_id,
                        label: format!("Authorize {}", device_label(snap, &device_path)),
                        device_path,
                        needs_pin: false,
                        needs_passkey: false,
                    },
                    BluetoothAgentRequest::AuthorizeService {
                        request_id,
                        device_path,
                        ..
                    } => PendingPairingPrompt {
                        request_id,
                        label: format!("Allow service for {}", device_label(snap, &device_path)),
                        device_path,
                        needs_pin: false,
                        needs_passkey: false,
                    },
                    BluetoothAgentRequest::RequestPinCode {
                        request_id,
                        device_path,
                    } => PendingPairingPrompt {
                        request_id,
                        label: format!("PIN requested (0000) for {}", device_label(snap, &device_path)),
                        device_path,
                        needs_pin: true,
                        needs_passkey: false,
                    },
                    BluetoothAgentRequest::RequestPasskey {
                        request_id,
                        device_path,
                    } => PendingPairingPrompt {
                        request_id,
                        label: format!("Passkey requested for {}", device_label(snap, &device_path)),
                        device_path,
                        needs_pin: false,
                        needs_passkey: true,
                    },
                    _ => return None,
                };
                self.pending_prompt = Some(prompt.clone());
                Some(prompt)
            }
            BluetoothAgentEvent::Cancelled | BluetoothAgentEvent::Released => {
                self.pending_prompt = None;
                None
            }
            _ => None,
        }
    }
}

fn device_label(snap: &BluetoothSnapshot, path: &str) -> String {
    snap.devices
        .iter()
        .find(|d| d.object_path == path)
        .map(|d| {
            if !d.alias.trim().is_empty() {
                d.alias.clone()
            } else if !d.name.trim().is_empty() {
                d.name.clone()
            } else {
                d.address.clone()
            }
        })
        .unwrap_or_else(|| path.rsplit('/').next().unwrap_or("Device").to_owned())
}
