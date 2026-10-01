//! Wireless Xbox controller batteries through XInput.

use windows::Win32::UI::Input::XboxController::{
    XInputGetBatteryInformation, BATTERY_DEVTYPE_GAMEPAD, BATTERY_TYPE_DISCONNECTED, BATTERY_TYPE_UNKNOWN,
    BATTERY_TYPE_WIRED, XINPUT_BATTERY_INFORMATION,
};

use super::{gamepad_percent, DeviceBattery};

pub fn read() -> Vec<DeviceBattery> {
    (0..4u32)
        .filter_map(|slot| {
            let mut info = XINPUT_BATTERY_INFORMATION::default();
            // SAFETY: `info` is a local of the expected type.
            let ok = unsafe { XInputGetBatteryInformation(slot, BATTERY_DEVTYPE_GAMEPAD, &mut info) } == 0;
            let wireless = ![
                BATTERY_TYPE_DISCONNECTED,
                BATTERY_TYPE_WIRED,
                BATTERY_TYPE_UNKNOWN,
            ]
            .contains(&info.BatteryType);
            (ok && wireless).then(|| DeviceBattery {
                name: if slot == 0 {
                    "Controller".into()
                } else {
                    format!("Controller {}", slot + 1)
                },
                kind: "controller".into(),
                percent: gamepad_percent(info.BatteryLevel.0),
            })
        })
        .collect()
}
