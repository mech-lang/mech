use gilrs::{Axis, Button, Gilrs};
use uuid::Uuid;

fn main() {
    let gilrs = Gilrs::new().unwrap();
    for (id, gamepad) in gilrs.gamepads() {
        println!(
            r#"Gamepad {id} ({name}):
    Map name: {map_name:?}
    Os name: {os_name}
    UUID: {uuid}
    Is connected: {is_connected}
    Power info: {power_info:?}
    Mapping source: {mapping_source:?}
    Is ff supported: {ff}
    Deadzone Left X: {dlx:?}
    Deadzone Left Y: {dly:?}
    Deadzone Right X: {drx:?}
    Deadzone Right Y: {dry:?}
    Deadzone Left Trigger: {dlt:?}
    Deadzone Right Trigger: {drt:?}
    Deadzone Left Trigger 2: {dlt2:?}
    Deadzone Right Trigger 2: {drt2:?}
"#,
            id = id,
            name = gamepad.name(),
            map_name = gamepad.map_name(),
            os_name = gamepad.os_name(),
            uuid = Uuid::from_bytes(gamepad.uuid()).to_hyphenated(),
            is_connected = gamepad.is_connected(),
            power_info = gamepad.power_info(),
            mapping_source = gamepad.mapping_source(),
            ff = gamepad.is_ff_supported(),
            dlx = gamepad
                .axis_code(Axis::LeftStickX)
                .and_then(|code| gamepad.deadzone(code)),
            dly = gamepad
                .axis_code(Axis::LeftStickY)
                .and_then(|code| gamepad.deadzone(code)),
            drx = gamepad
                .axis_code(Axis::RightStickX)
                .and_then(|code| gamepad.deadzone(code)),
            dry = gamepad
                .axis_code(Axis::RightStickY)
                .and_then(|code| gamepad.deadzone(code)),
            dlt = gamepad
                .button_code(Button::LeftTrigger)
                .and_then(|code| gamepad.deadzone(code)),
            drt = gamepad
                .button_code(Button::RightTrigger)
                .and_then(|code| gamepad.deadzone(code)),
            dlt2 = gamepad
                .button_code(Button::LeftTrigger2)
                .and_then(|code| gamepad.deadzone(code)),
            drt2 = gamepad
                .button_code(Button::RightTrigger2)
                .and_then(|code| gamepad.deadzone(code)),
        );
    }
}
