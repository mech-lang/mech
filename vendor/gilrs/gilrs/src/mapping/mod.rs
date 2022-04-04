// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![cfg_attr(target_os = "windows", allow(dead_code))]

mod parser;

use crate::ev::{self, Axis, AxisOrBtn, Button};
use gilrs_core::native_ev_codes as nec;
use gilrs_core::EvCode;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use fnv::FnvHashMap;
use uuid::Uuid;
use vec_map::VecMap;

use self::parser::{Error as ParserError, ErrorKind as ParserErrorKind, Parser, Token};

/// Platform name used by SDL mappings
#[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd"))]
const SDL_PLATFORM_NAME: &str = "Linux";
#[cfg(target_os = "macos")]
const SDL_PLATFORM_NAME: &'static str = "Mac OS X";
#[cfg(target_os = "windows")]
const SDL_PLATFORM_NAME: &'static str = "Windows";
#[cfg(all(
    not(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd")),
    not(target_os = "macos"),
    not(target_os = "windows")
))]
const SDL_PLATFORM_NAME: &'static str = "Unknown";

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
/// Store mappings from one `EvCode` (`u16`) to another.
///
/// This struct is internal, `MappingData` is exported in public interface as `Mapping`.
pub struct Mapping {
    mappings: FnvHashMap<EvCode, AxisOrBtn>,
    name: String,
    default: bool,
    hats_mapped: u8,
}

impl Mapping {
    pub fn new() -> Self {
        Mapping {
            mappings: FnvHashMap::default(),
            name: String::new(),
            default: false,
            hats_mapped: 0,
        }
    }

    pub fn default(gamepad: &gilrs_core::Gamepad) -> Self {
        use self::Axis as Ax;
        use self::AxisOrBtn::*;

        macro_rules! fnv_map {
            ( $( $key:expr => $elem:expr ),* ) => {
                {
                    let mut map = FnvHashMap::default();
                    $(
                        map.insert($key, $elem);
                    )*

                    map
                }
            };
        }

        let mut mappings = fnv_map![
            nec::BTN_SOUTH => Btn(Button::South),
            nec::BTN_EAST => Btn(Button::East),
            nec::BTN_C => Btn(Button::C),
            nec::BTN_NORTH => Btn(Button::North),
            nec::BTN_WEST => Btn(Button::West),
            nec::BTN_Z => Btn(Button::Z),
            nec::BTN_LT => Btn(Button::LeftTrigger),
            nec::BTN_RT => Btn(Button::RightTrigger),
            nec::BTN_LT2 => Btn(Button::LeftTrigger2),
            nec::BTN_RT2 => Btn(Button::RightTrigger2),
            nec::BTN_SELECT => Btn(Button::Select),
            nec::BTN_START => Btn(Button::Start),
            nec::BTN_MODE => Btn(Button::Mode),
            nec::BTN_LTHUMB => Btn(Button::LeftThumb),
            nec::BTN_RTHUMB => Btn(Button::RightThumb),
            nec::BTN_DPAD_UP => Btn(Button::DPadUp),
            nec::BTN_DPAD_DOWN => Btn(Button::DPadDown),
            nec::BTN_DPAD_LEFT => Btn(Button::DPadLeft),
            nec::BTN_DPAD_RIGHT => Btn(Button::DPadRight),

            nec::AXIS_LT => Btn(Button::LeftTrigger),
            nec::AXIS_RT => Btn(Button::RightTrigger),
            nec::AXIS_LT2 => Btn(Button::LeftTrigger2),
            nec::AXIS_RT2 => Btn(Button::RightTrigger2),

            nec::AXIS_LSTICKX => Axis(Ax::LeftStickX),
            nec::AXIS_LSTICKY => Axis(Ax::LeftStickY),
            nec::AXIS_LEFTZ => Axis(Ax::LeftZ),
            nec::AXIS_RSTICKX => Axis(Ax::RightStickX),
            nec::AXIS_RSTICKY => Axis(Ax::RightStickY),
            nec::AXIS_RIGHTZ => Axis(Ax::RightZ),
            nec::AXIS_DPADX => Axis(Ax::DPadX),
            nec::AXIS_DPADY => Axis(Ax::DPadY)
        ];

        // Remove all mappings that don't have corresponding element in gamepad. Partial fix to #83
        let axes = [
            nec::AXIS_DPADX,
            nec::AXIS_DPADY,
            nec::AXIS_LEFTZ,
            nec::AXIS_LSTICKX,
            nec::AXIS_LSTICKY,
            nec::AXIS_RSTICKX,
            nec::AXIS_RSTICKY,
            nec::AXIS_LT,
            nec::AXIS_LT2,
            nec::AXIS_RT,
            nec::AXIS_RT2,
            nec::AXIS_RIGHTZ,
        ];
        let btns = [
            nec::BTN_SOUTH,
            nec::BTN_NORTH,
            nec::BTN_WEST,
            nec::BTN_WEST,
            nec::BTN_C,
            nec::BTN_Z,
            nec::BTN_LT,
            nec::BTN_LT2,
            nec::BTN_RT,
            nec::BTN_RT2,
            nec::BTN_SELECT,
            nec::BTN_START,
            nec::BTN_MODE,
            nec::BTN_LTHUMB,
            nec::BTN_RTHUMB,
            nec::BTN_DPAD_DOWN,
            nec::BTN_DPAD_LEFT,
            nec::BTN_DPAD_RIGHT,
            nec::BTN_DPAD_UP,
        ];

        for axis in &axes {
            if !gamepad.axes().contains(axis) {
                mappings.remove(axis);
            }
        }

        for btn in &btns {
            if !gamepad.buttons().contains(btn) {
                mappings.remove(btn);
            }
        }

        Mapping {
            mappings,
            name: String::new(),
            default: true,
            hats_mapped: 0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn from_data(
        data: &MappingData,
        buttons: &[EvCode],
        axes: &[EvCode],
        name: &str,
        uuid: Uuid,
    ) -> Result<(Self, String), MappingError> {
        use crate::constants::*;

        if !Self::is_name_valid(name) {
            return Err(MappingError::InvalidName);
        }

        let mut mappings = FnvHashMap::default();
        let mut sdl_mappings = format!("{},{},", uuid.to_simple(), name);

        {
            let mut add_button = |ident, ev_code, mapped_btn| {
                Self::add_button(
                    ident,
                    ev_code,
                    mapped_btn,
                    buttons,
                    &mut sdl_mappings,
                    &mut mappings,
                )
            };

            for (button, &ev_code) in &data.buttons {
                match button as u16 {
                    BTN_SOUTH => add_button("a", ev_code, Button::South)?,
                    BTN_EAST => add_button("b", ev_code, Button::East)?,
                    BTN_WEST => add_button("x", ev_code, Button::West)?,
                    BTN_NORTH => add_button("y", ev_code, Button::North)?,
                    BTN_LT => add_button("leftshoulder", ev_code, Button::LeftTrigger)?,
                    BTN_RT => add_button("rightshoulder", ev_code, Button::RightTrigger)?,
                    BTN_LT2 => add_button("lefttrigger", ev_code, Button::LeftTrigger2)?,
                    BTN_RT2 => add_button("righttrigger", ev_code, Button::RightTrigger2)?,
                    BTN_SELECT => add_button("back", ev_code, Button::Select)?,
                    BTN_START => add_button("start", ev_code, Button::Start)?,
                    BTN_MODE => add_button("guide", ev_code, Button::Mode)?,
                    BTN_LTHUMB => add_button("leftstick", ev_code, Button::LeftThumb)?,
                    BTN_RTHUMB => add_button("rightstick", ev_code, Button::RightThumb)?,
                    BTN_DPAD_UP => add_button("dpup", ev_code, Button::DPadUp)?,
                    BTN_DPAD_DOWN => add_button("dpdown", ev_code, Button::DPadDown)?,
                    BTN_DPAD_LEFT => add_button("dpleft", ev_code, Button::DPadLeft)?,
                    BTN_DPAD_RIGHT => add_button("dpright", ev_code, Button::DPadRight)?,
                    BTN_C => add_button("c", ev_code, Button::C)?,
                    BTN_Z => add_button("z", ev_code, Button::Z)?,
                    BTN_UNKNOWN => return Err(MappingError::UnknownElement),
                    _ => unreachable!(),
                }
            }
        }

        {
            let mut add_axis = |ident, ev_code, mapped_axis| {
                Self::add_axis(
                    ident,
                    ev_code,
                    mapped_axis,
                    axes,
                    &mut sdl_mappings,
                    &mut mappings,
                )
            };

            for (axis, &ev_code) in &data.axes {
                match axis as u16 {
                    AXIS_LSTICKX => add_axis("leftx", ev_code, Axis::LeftStickX)?,
                    AXIS_LSTICKY => add_axis("lefty", ev_code, Axis::LeftStickY)?,
                    AXIS_RSTICKX => add_axis("rightx", ev_code, Axis::RightStickX)?,
                    AXIS_RSTICKY => add_axis("righty", ev_code, Axis::RightStickY)?,
                    AXIS_LEFTZ => add_axis("leftz", ev_code, Axis::LeftZ)?,
                    AXIS_RIGHTZ => add_axis("rightz", ev_code, Axis::RightZ)?,
                    AXIS_UNKNOWN => return Err(MappingError::UnknownElement),
                    _ => unreachable!(),
                }
            }
        }

        let mapping = Mapping {
            mappings,
            name: name.to_owned(),
            default: false,
            hats_mapped: 0,
        };

        Ok((mapping, sdl_mappings))
    }

    pub fn parse_sdl_mapping(
        line: &str,
        buttons: &[EvCode],
        axes: &[EvCode],
    ) -> Result<Self, ParseSdlMappingError> {
        let mut mapping = Mapping::new();
        let mut parser = Parser::new(line);

        while let Some(token) = parser.next_token() {
            if let Err(ref e) = token {
                if e.kind() == &ParserErrorKind::EmptyValue {
                    continue;
                }
            }

            let token = token?;

            match token {
                Token::Platform(platform) => {
                    if platform != SDL_PLATFORM_NAME {
                        warn!("Mappings for different platform – {}", platform);
                    }
                }
                Token::Uuid(_) => (),
                Token::Name(name) => mapping.name = name.to_owned(),
                Token::AxisMapping { from, to, .. } => {
                    let axis = axes
                        .get(from as usize)
                        .cloned()
                        .ok_or(ParseSdlMappingError::InvalidAxis)?;
                    mapping.mappings.insert(axis, to);
                }
                Token::ButtonMapping { from, to } => {
                    let btn = buttons
                        .get(from as usize)
                        .cloned()
                        .ok_or(ParseSdlMappingError::InvalidButton)?;
                    mapping.mappings.insert(btn, AxisOrBtn::Btn(to));
                }
                Token::HatMapping { hat, direction, to } => {
                    if hat != 0 || !to.is_dpad() {
                        warn!(
                            "Hat mappings are only supported for dpads (requested to map hat \
                             {}.{} to {:?}",
                            hat, direction, to
                        );
                    } else {
                        // We  don't have anything like "hat" in gilrs, so let's jus assume that
                        // user want to map dpad axes.
                        //
                        // We have to add mappings for axes AND buttons, because axis_dpad_to_button
                        // filter may transform event to button event.
                        let (from_axis, from_btn) = match direction {
                            1 => (nec::AXIS_DPADY, nec::BTN_DPAD_UP),
                            4 => (nec::AXIS_DPADY, nec::BTN_DPAD_DOWN),
                            2 => (nec::AXIS_DPADX, nec::BTN_DPAD_RIGHT),
                            8 => (nec::AXIS_DPADX, nec::BTN_DPAD_LEFT),
                            0 => continue, // FIXME: I have no idea what 0 means here
                            _ => return Err(ParseSdlMappingError::UnknownHatDirection),
                        };

                        let to_axis = match to {
                            Button::DPadLeft | Button::DPadRight => Axis::DPadX,
                            Button::DPadUp | Button::DPadDown => Axis::DPadY,
                            _ => unreachable!(),
                        };

                        mapping.mappings.insert(from_axis, AxisOrBtn::Axis(to_axis));
                        mapping.mappings.insert(from_btn, AxisOrBtn::Btn(to));
                        mapping.hats_mapped |= direction as u8;
                    }
                }
            }
        }

        Ok(mapping)
    }

    fn add_button(
        ident: &str,
        ev_code: EvCode,
        mapped_btn: Button,
        buttons: &[EvCode],
        sdl_mappings: &mut String,
        mappings: &mut FnvHashMap<EvCode, AxisOrBtn>,
    ) -> Result<(), MappingError> {
        let n_btn = buttons
            .iter()
            .position(|&x| x == ev_code)
            .ok_or(MappingError::InvalidCode(ev::Code(ev_code)))?;
        sdl_mappings.push_str(&format!("{}:b{},", ident, n_btn));
        mappings.insert(ev_code, AxisOrBtn::Btn(mapped_btn));
        Ok(())
    }

    fn add_axis(
        ident: &str,
        ev_code: EvCode,
        mapped_axis: Axis,
        axes: &[EvCode],
        sdl_mappings: &mut String,
        mappings: &mut FnvHashMap<EvCode, AxisOrBtn>,
    ) -> Result<(), MappingError> {
        let n_axis = axes
            .iter()
            .position(|&x| x == ev_code)
            .ok_or(MappingError::InvalidCode(ev::Code(ev_code)))?;
        sdl_mappings.push_str(&format!("{}:a{},", ident, n_axis));
        mappings.insert(ev_code, AxisOrBtn::Axis(mapped_axis));
        Ok(())
    }

    fn is_name_valid(name: &str) -> bool {
        !name.chars().any(|x| x == ',')
    }

    pub fn map(&self, code: &EvCode) -> Option<AxisOrBtn> {
        self.mappings.get(code).cloned()
    }

    pub fn map_rev(&self, el: &AxisOrBtn) -> Option<EvCode> {
        self.mappings.iter().find(|x| x.1 == el).map(|x| *x.0)
    }

    pub fn is_default(&self) -> bool {
        self.default
    }

    /// Return bit field with mapped hats. Only for mappings created from SDL format this function
    /// can return non-zero value.
    pub fn hats_mapped(&self) -> u8 {
        self.hats_mapped
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum ParseSdlMappingError {
    InvalidButton,
    InvalidAxis,
    UnknownHatDirection,
    ParseError(ParserError),
}

impl From<ParserError> for ParseSdlMappingError {
    fn from(f: ParserError) -> Self {
        ParseSdlMappingError::ParseError(f)
    }
}

impl Error for ParseSdlMappingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let ParseSdlMappingError::ParseError(ref err) = self {
            Some(err)
        } else {
            None
        }
    }
}

impl Display for ParseSdlMappingError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
        let s = match self {
            ParseSdlMappingError::InvalidButton => "gamepad doesn't have requested button",
            ParseSdlMappingError::InvalidAxis => "gamepad doesn't have requested axis",
            ParseSdlMappingError::UnknownHatDirection => "hat direction wasn't 1, 2, 4 or 8",
            ParseSdlMappingError::ParseError(_) => "parsing error",
        };

        fmt.write_str(s)
    }
}

#[derive(Debug)]
pub struct MappingDb {
    mappings: HashMap<Uuid, String>,
}

impl MappingDb {
    pub fn new() -> Self {
        MappingDb {
            mappings: HashMap::new(),
        }
    }

    pub fn add_included_mappings(&mut self) {
        self.insert(include_str!(
            "../../SDL_GameControllerDB/gamecontrollerdb.txt"
        ));
    }

    pub fn add_env_mappings(&mut self) {
        if let Ok(mapping) = env::var("SDL_GAMECONTROLLERCONFIG") {
            self.insert(&mapping);
        }
    }

    pub fn insert(&mut self, s: &str) {
        for mapping in s.lines() {
            let pat = "platform:";
            if let Some(offset) = mapping.find(pat).map(|o| o + pat.len()) {
                let s = &mapping[offset..];
                let end = s.find(',').unwrap_or_else(|| s.len());

                if &s[..end] != SDL_PLATFORM_NAME {
                    continue;
                }
            }

            mapping
                .split(',')
                .next()
                .and_then(|s| Uuid::parse_str(s).ok())
                .and_then(|uuid| self.mappings.insert(uuid, mapping.to_owned()));
        }
    }

    pub fn get(&self, uuid: Uuid) -> Option<&str> {
        self.mappings.get(&uuid).map(String::as_ref)
    }

    pub fn len(&self) -> usize {
        self.mappings.len()
    }
}

/// Stores data used to map gamepad buttons and axes.
///
/// After you add all mappings, use
/// [`Gamepad::set_mapping(…)`](struct.Gamepad.html#method.set_mapping) to change mapping of
/// existing gamepad.
///
/// See `examples/mapping.rs` for more detailed example.
#[derive(Debug, Clone, Default)]
// Re-exported as Mapping
pub struct MappingData {
    buttons: VecMap<EvCode>,
    axes: VecMap<EvCode>,
}

impl MappingData {
    /// Creates new `Mapping`.
    pub fn new() -> Self {
        MappingData {
            buttons: VecMap::with_capacity(18),
            axes: VecMap::with_capacity(11),
        }
    }

    /// Returns `EvCode` associated with button index.
    pub fn button(&self, idx: Button) -> Option<ev::Code> {
        self.buttons.get(idx as usize).cloned().map(ev::Code)
    }

    /// Returns `EvCode` associated with axis index.
    pub fn axis(&self, idx: Axis) -> Option<ev::Code> {
        self.axes.get(idx as usize).cloned().map(ev::Code)
    }

    /// Inserts new button mapping.
    pub fn insert_btn(&mut self, from: ev::Code, to: Button) -> Option<ev::Code> {
        self.buttons.insert(to as usize, from.0).map(ev::Code)
    }

    /// Inserts new axis mapping.
    pub fn insert_axis(&mut self, from: ev::Code, to: Axis) -> Option<ev::Code> {
        self.axes.insert(to as usize, from.0).map(ev::Code)
    }

    /// Removes button and returns associated `NativEvCode`.
    pub fn remove_button(&mut self, idx: Button) -> Option<ev::Code> {
        self.buttons.remove(idx as usize).map(ev::Code)
    }

    /// Removes axis and returns associated `NativEvCode`.
    pub fn remove_axis(&mut self, idx: Axis) -> Option<ev::Code> {
        self.axes.remove(idx as usize).map(ev::Code)
    }
}

/// The error type for functions related to gamepad mapping.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MappingError {
    /// Gamepad does not have element referenced by `EvCode`.
    InvalidCode(ev::Code),
    /// Name contains comma (',').
    InvalidName,
    /// This function is not implemented for current platform.
    NotImplemented,
    /// Gamepad is not connected.
    NotConnected,
    /// Same gamepad element is referenced by axis and button.
    DuplicatedEntry,
    /// `Mapping` with `Button::Unknown` or `Axis::Unknown`.
    UnknownElement,
    /// `Mapping` have button or axis that are not present in SDL2.
    NotSdl2Compatible,
}

impl Error for MappingError {}

impl Display for MappingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let sbuf;
        let s = match self {
            MappingError::InvalidCode(code) => {
                sbuf = format!("gamepad does not have element with {}", code);
                sbuf.as_ref()
            }
            MappingError::InvalidName => "name can not contain comma",
            MappingError::NotImplemented => {
                "current platform does not implement setting custom mappings"
            }
            MappingError::NotConnected => "gamepad is not connected",
            MappingError::DuplicatedEntry => {
                "same gamepad element is referenced by axis and button"
            }
            MappingError::UnknownElement => "Button::Unknown and Axis::Unknown are not allowed",
            MappingError::NotSdl2Compatible => "one of buttons or axes is not compatible with SDL2",
        };

        f.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ev::{Axis, Button};
    use gilrs_core::native_ev_codes as nec;
    use gilrs_core::EvCode;
    use uuid::Uuid;
    // Do not include platform, mapping from (with UUID modified)
    // https://github.com/gabomdq/SDL_GameControllerDB/blob/master/gamecontrollerdb.txt
    const TEST_STR: &str = "03000000260900008888000000010001,GameCube {WiseGroup USB \
                            box},a:b0,b:b2,y:b3,x:b1,start:b7,rightshoulder:b6,dpup:h0.1,dpleft:\
                            h0.8,dpdown:h0.4,dpright:h0.2,leftx:a0,lefty:a1,rightx:a2,righty:a3,\
                            lefttrigger:a4,righttrigger:a5,";

    const BUTTONS: [EvCode; 15] = [
        nec::BTN_SOUTH,
        nec::BTN_EAST,
        nec::BTN_C,
        nec::BTN_NORTH,
        nec::BTN_WEST,
        nec::BTN_Z,
        nec::BTN_LT,
        nec::BTN_RT,
        nec::BTN_LT2,
        nec::BTN_RT2,
        nec::BTN_SELECT,
        nec::BTN_START,
        nec::BTN_MODE,
        nec::BTN_LTHUMB,
        nec::BTN_RTHUMB,
    ];

    const AXES: [EvCode; 12] = [
        nec::AXIS_LSTICKX,
        nec::AXIS_LSTICKY,
        nec::AXIS_LEFTZ,
        nec::AXIS_RSTICKX,
        nec::AXIS_RSTICKY,
        nec::AXIS_RIGHTZ,
        nec::AXIS_DPADX,
        nec::AXIS_DPADY,
        nec::AXIS_RT,
        nec::AXIS_LT,
        nec::AXIS_RT2,
        nec::AXIS_LT2,
    ];

    #[test]
    fn mapping() {
        Mapping::parse_sdl_mapping(TEST_STR, &BUTTONS, &AXES).unwrap();
    }

    #[test]
    fn from_data() {
        let uuid = Uuid::nil();
        let name = "Best Gamepad";
        let buttons = BUTTONS.iter().cloned().map(ev::Code).collect::<Vec<_>>();
        let axes = AXES.iter().cloned().map(ev::Code).collect::<Vec<_>>();

        let mut data = MappingData::new();
        data.insert_axis(axes[0], Axis::LeftStickX);
        data.insert_axis(axes[1], Axis::LeftStickY);
        data.insert_axis(axes[2], Axis::LeftZ);
        data.insert_axis(axes[3], Axis::RightStickX);
        data.insert_axis(axes[4], Axis::RightStickY);
        data.insert_axis(axes[5], Axis::RightZ);

        data.insert_btn(buttons[0], Button::South);
        data.insert_btn(buttons[1], Button::East);
        data.insert_btn(buttons[3], Button::North);
        data.insert_btn(buttons[4], Button::West);
        data.insert_btn(buttons[5], Button::Select);
        data.insert_btn(buttons[6], Button::Start);
        data.insert_btn(buttons[7], Button::DPadDown);
        data.insert_btn(buttons[8], Button::DPadLeft);
        data.insert_btn(buttons[9], Button::RightThumb);

        let (mappings, sdl_mappings) =
            Mapping::from_data(&data, &BUTTONS, &AXES, name, uuid).unwrap();
        let sdl_mappings = Mapping::parse_sdl_mapping(&sdl_mappings, &BUTTONS, &AXES).unwrap();
        assert_eq!(mappings, sdl_mappings);

        let incorrect_mappings = Mapping::from_data(&data, &BUTTONS, &AXES, "Inval,id name", uuid);
        assert_eq!(Err(MappingError::InvalidName), incorrect_mappings);

        data.insert_btn(ev::Code(nec::BTN_DPAD_RIGHT), Button::DPadRight);
        let incorrect_mappings = Mapping::from_data(&data, &BUTTONS, &AXES, name, uuid);
        assert_eq!(
            Err(MappingError::InvalidCode(ev::Code(nec::BTN_DPAD_RIGHT))),
            incorrect_mappings
        );

        data.insert_btn(ev::Code(BUTTONS[3]), Button::Unknown);
        let incorrect_mappings = Mapping::from_data(&data, &BUTTONS, &AXES, name, uuid);
        assert_eq!(Err(MappingError::UnknownElement), incorrect_mappings);
    }

    #[test]
    fn with_mappings() {
        let mappings = format!(
            "\nShould be ignored\nThis also should,be ignored\n\n{}",
            TEST_STR
        );
        let mut db = MappingDb::new();
        db.add_included_mappings();
        db.insert(&mappings);

        assert_eq!(
            Some(TEST_STR),
            db.get(Uuid::parse_str("03000000260900008888000000010001").unwrap())
        );
    }
}
