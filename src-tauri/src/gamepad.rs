use gilrs::{Axis, Button, EventType, Gilrs};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(tag = "kind")]
pub enum GamepadEvent {
    #[serde(rename = "button")]
    Button { name: &'static str, pressed: bool },
    #[serde(rename = "axis")]
    Axis { name: &'static str, value: f32 },
}

fn button_name(btn: Button) -> Option<&'static str> {
    match btn {
        Button::South => Some("A"),
        Button::East => Some("B"),
        Button::West => Some("X"),
        Button::North => Some("Y"),
        Button::LeftTrigger => Some("L1"),
        Button::RightTrigger => Some("R1"),
        Button::LeftTrigger2 => Some("L2"),
        Button::RightTrigger2 => Some("R2"),
        Button::DPadUp => Some("DPadUp"),
        Button::DPadDown => Some("DPadDown"),
        Button::DPadLeft => Some("DPadLeft"),
        Button::DPadRight => Some("DPadRight"),
        Button::Start => Some("Start"),
        Button::Select => Some("Select"),
        _ => None,
    }
}

fn axis_name(axis: Axis) -> Option<&'static str> {
    match axis {
        Axis::LeftStickX => Some("LeftStickX"),
        Axis::LeftStickY => Some("LeftStickY"),
        Axis::RightStickX => Some("RightStickX"),
        Axis::RightStickY => Some("RightStickY"),
        _ => None,
    }
}

/// Spawn a background thread that polls gamepads via gilrs and emits
/// `gamepad-event` Tauri events to the frontend webview.
pub fn start(handle: AppHandle) {
    std::thread::spawn(move || {
        let mut gilrs = match Gilrs::new() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("[DeckSave] gilrs init failed: {e}");
                return;
            }
        };

        eprintln!(
            "[DeckSave] Gamepad backend started – {} gamepad(s) connected",
            gilrs.gamepads().count()
        );
        for (_id, gp) in gilrs.gamepads() {
            eprintln!("[DeckSave]   • {}", gp.name());
        }

        loop {
            // Process queued events (button press/release, axis changes)
            while let Some(gilrs::Event { event, .. }) = gilrs.next_event() {
                let payload = match event {
                    EventType::ButtonPressed(btn, _) => {
                        button_name(btn).map(|name| GamepadEvent::Button { name, pressed: true })
                    }
                    EventType::ButtonReleased(btn, _) => {
                        button_name(btn).map(|name| GamepadEvent::Button {
                            name,
                            pressed: false,
                        })
                    }
                    EventType::AxisChanged(axis, value, _) => {
                        axis_name(axis).map(|name| GamepadEvent::Axis { name, value })
                    }
                    _ => None,
                };

                if let Some(ev) = payload {
                    let _ = handle.emit("gamepad-event", &ev);
                }
            }

            // ~125 Hz poll rate – responsive without burning CPU
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
    });
}
