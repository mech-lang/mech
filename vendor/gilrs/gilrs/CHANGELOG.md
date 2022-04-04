Change Log
==========

v0.8.2 - 2021-12-30
-------------------

### Changed

- Minimal supported rust version is now 1.47
- `axis_dpad_to_btn` now also emits `ButtonChanged` events

### Fixed

- Fixed overflow when calculating axis value and min/max range was
  i32::MIN/MAX (@wathiede)


v0.8.1 - 2021-03-30
-------------------

### Changed

- Updated bundled mappings

v0.8.0 - 2020-10-09
-------------------

### Added

- `Jitter`, `Repeat`, `GilrsBuilder`, and `Mapping` now implement `Default`.
- Errors now implement `source()`.
- `Code` now implements `Deserialize` and `Serialize` (@theunkn0wn1).
- Dpad is now supported on macOS (@cleancut).

### Changed

- Minimal supported version is now 1.40
- Non exhaustive enums now use `#[non_exhaustive]` instead of hidden variant.
- Renamed cargo feature `serde` to `serde-serialize`.
- Improved conversion of axis value to float. Values like 127 (when axis range
  is 0-255) will now be correctly converted to 0.0.

### Removed

- Errors now longer implement deprecated methods (`source()` and `description()`).

v0.7.4 - 2020-02-06
-------------------

### Added

- Added method to stop playing force feedback effect. (@photex)

### Fixed

- Fixed bug that caused forced feedback effects to never stop. (@photex)

v0.7.3 - 2019-11-30
-------------------

### Added

- Added support for serialization and deserialization for `Button`, `Axis`
  and `AxisOrButton` with optional `serde` feature (@aleksijuvani).

### Fixed

- Fixed defaults mappings containing elements that gamepad doesn't have.
  This also fixes state not working for `LeftTrigger`  button on Windows.

v0.7.2 - 2019-08-06
-------------------

### Fixed

- Fixed loading mappings for wrong platform

v0.7.1 - 2019-03-04
-------------------

### Fixed

- Compilation on macOS.
- xinput: Calling `set_ff_state()` on devices that were never connected.
- `GamepadId` was not reexported from private module.

v0.7.0 - 2019-02-21
-------------------

### Added

- `GamepadId`
- `Gilrs::gamepad(id)`. This function is replacement
  for `Index` operator and can return disconnected gamepads.
- Initial support for macOS (@jtakakura). There are still some functionality
  missing, check related issues in #58.
- Wasm support, using stdweb (@ryanisaacg).

### Changed

- Change `Gamepad::uuid -> Uuid` to `Gamepad::uuid -> [u8; 16]`
- `gilrs` now uses `gilrs-core` crate as backend. Because of it,
  there are some breaking changes to API.
- Functions that returned `&Gamepad` now return `Gamepad<'_>` proxy object.
- Renamed `Gilrs::get(id)` to `Gilrs::connected_gamepad(id)`.
- Moved `Gamepad::set_mapping{,_strict}()` to `Gilrs`. These functions now
  also take gamepad id as additional argument.
- Minimal supported version is now 1.31.1. The crate can still be build with
  older rustc, but it may change during next patch release.
- Instead using `usize` for gamepad ID, `GamepadId` is now used.
- Updated bundled SDL_GameControllerDB.

### Removed

- All functions that returned `&mut Gamepad`.
- `Gilrs` no longer implements `Index` and `IndexMut` operators. Use
  `Gilrs::gamepad(id)` instead.
- `Gamepad::status()` and `Status` enum. `Gamepad::is_connected()` is
  now sufficient to determine status of gamepad.

### Fixed

- xinput: Incorrect gamepad ID when more than one gamepad is connected
  (@DTibbs).
- Deadzone filter no longer emits additional events. This resulted in emitting
  more events until values normalized on some, often unrelated (like 0 for axis
  around 0.5), value.
- Mappings from environment variable had lower priority than bundled mappings.

v0.6.1 - 2018-07-18
-------------------

### Added

- `ev::Code::into_u32()` (@rukai).
- `ev::{Button, Axis, AxisOrBtn}` now implements `Hash` (@sheath).

### Changed

- The URL of repository has changed to https://gitlab.com/gilrs-project/gilrs
- Updated bundled SDL_GameControllerDB.

### Fixed

- Various fixes to logging at incorrect log level. Thanks to @fuggles for
  locating and reporting these issues.
- Possible panic in `Repeat` filter.
- `Axis::DPadY` was inverted on Linux.

v0.6.0 - 2018-02-11
-------------------

### Added

- Support for parsing SLD 2.0.6 mappings.
- `ButtonChanged` event. It contains value in range [0.0, 1.0].
- `GilrsBuilder::set_axis_to_btn()`. It allow to customize on which values
  `ButtonePressed` and `ButtonReleased` are emitted.
- `GilrsBuilder::set_update_state` which control whether gamepad state should
  be updated automatically.
- `ButtonState::value()`.
- `Mapping::insert_{btn,axis}()`.
- `Gampead::os_name()` and `Gamepad::map_name()`. (@rukai)
- `GilrsBuilder::add_env_mappings()` and `GilrsBuilder::add_included_mappings()`,
  allow to configure whether to load mappings from `SDL_GAMECONTROLLERCONFIG` env
  and bundled mappings. (@rukai)
- `Gilrs::insert_event()`.
- `Axis::second_axis()` – returns the other axis of gamepad element. For example,
  this function will return `LeftStickX` for `LeftStickY`.

### Removed

- `Mapping` no longer implements `Index` and `IndexMut` operators. Use
  `Mapping::insert_{btn,axis}()` methods to add new mappings.
- `Axis::{LeftTrigger, LeftTrigger2, RightTrigger, RightTrigger2}`. All events
  with these are now button events. `ButtonChanged` event contains value.
- `Gilrs::gamepad()` and `Gilrs::gamepad_mut()` – use `Index` operator instead.

### Changed

- Gilrs now require Rust 1.20.0 or newer.
- Updated bundled mappings.
- Renamed `Filter::filter` to `Filter::filter_ev` because RFC 2124 added
  `filter` method to `Option` (our `Filter` is implemented for `Option<Event>`).
- `Gamepad::deadzone()` now returns `Option<f32>` instead of `f32`.
- All axis events are now in range [-1.0, 1.0].
- `NativeEvCode` is replaced by `ev::Code`, a strongly typed struct that also
  distinguish between axes and buttons.
- You can now create mappings from any axis to any button.
- `State` now tracks floating-point value of buttons.
- `State::value()` can now be used to also examine value of buttons.
- By default, gamepad state is updated automatically. If you customize event
  filters, you can disable this behaviour using `GilrsBuilder::set_update_state`.
- `Gilrs::new()` and `GilrsBuilder::build()` now returns `Result`. Dummy context
  can still be used, but only if result of failure is unsupported platform.
- Renamed `Gilrs::connected_gamepad()` and `Gilrs::connected_gamepad_mut()` to
  `get()` and `get_mut()`.
- `Filter` and `FilterFn` now borrows `Gilrs` mutably.
- Windows: Gamepads are now named "Xbox Controller" instead of "XInput Controller".
  (@rukai)

### Fixed

- Incorrect ranges for some axes.
- Deadzone filter should no longer produce values outside of allowed range.
- When calculating deadzone, the value of second axis is no longer ignored.
  This fixes situation, when sometimes axis would stay on value small to 0.0,
  when it should be 0.0 instead.
- Deadzone threshold was half of what it should be.
- Linux: Fixed axis value normalization if neither minimal value is 0 nor
  midpoint is 0. (@scottpleb)
- Linux: Ensure that axis values are clamped after normalization. (@scottpleb)
- Linux: Compilation error on architectures with `c_char = u8`.

v0.5.0 - 2017-09-24
-------------------

### Added

- `Mapping::remove_button()` and `Mapping::remove_axis()`.
- `GilrsBuilder` for customizing how `Gilrs` is created.
- Event filters. See `ev::filter` module for more info.
- `Gilrs::next_event()` - use it with `while let` loop in your event loop.
  This allow to avoid borrow checker problems that `EventIterator` caused.
- New event – `Dropped`. Used by filters to indicate that you should ignore
  this event.
- New event – `ButtonRepeated`. Can be emitted by `Repeat` filter.
- `Axis::{DPadX, DPadY}`
- `Gamepad::{button_name, axis_name, button_code, axis_code}` functions for
  accessing mapping data.
- `Gamepad::axis_data, button_data` – part of new extended gamepad state.
- `Gamepad::id()` – returns gamepad ID.
- `Gilrs::update, inc, counter, reset_counter` – part of new extended
   gamepad state.

### Removed

- `Gilrs::with_mappings()` – use `GilrsBuilder`.
- `Gilrs::poll_events()` and `EventIterator` – use `Gilrs::next_event()`
  instead.

### Changed

- Minimal rust version is now 1.19
- New gamepad state. Now can store state for any button or axis (previously was
  only useful for named buttons and axes). Additionally it now also know when
  last event happened. Basic usage with `is_pressed()` and `value()` methods is
  same, but check out documentation for new features.
- Gamepad state now must be explicitly updated with `Gilrs::update(Event)`.
  This change was necessary because filters can change events.
- `Event` is now a struct and contains common information like id of gamepad
  and timestamp (new). Old enum was renamed to `EventType` and can be accessed
  from `Event.event` public field.
- New force feedback module, including support for Windows. There are to many
  changes to list them all here, so pleas check documentation and examples.
- Renamed `ff::Error::EffectNotSupported` to `ff::Error::NotSupported`.
- `Button::Unknown` and `Axis::Unknown` have now value of 0.
- `Gamepad::set_mapping()` (and `_strict` variant) now returns error when
  creating mapping with `Button::Unknown` or `Axis::Unknown`. Additionally
  `_strict` version does not allow `Button::{C, Z}` and Axis::{LeftZ, RightZ}.
- xinput: New values for `NativEvCode`

### Fixed

- Panic on `unreachable!()` when creating mapping with `Button::{C, Z,
  Unknown}` or `Axis::{LeftZ, RightZ}`.

v0.4.4 — 2017-06-16
-------------------

### Changed

- Gilrs no longer uses `ioctl` crate on Linux. Because `ioctl` was deprecated
  and all versions yanked, it was causing problems for new builds that didn't
  have `ioctl` crate listed in Cargo.lock.

v0.4.3 — 2017-03-12
-------------------

### Added

- You can now iterate over mutable references to connected gamepads using
  `Gilrs::gamepads_mut()`.

### Fixed

- Fixed `unreachable!()` panic on 32bit Linux
- Improved converting axes values to `f32` when using XInput

v0.4.2 - 2017-01-15
-------------------

### Changed

- Updated SDL_GameControllerDB to latest revision.
- Changes in axes values that are less than 1% are now ignored.

### Fixed

- Fixed multiple axes mapped to same axis name when mappings are incomplete.
- Values returned with `AxisChanged` event now have correctly applied
  deadzones.
- Linux: Correctly handle event queue overrun.


v0.4.1 - 2016-12-12
-------------------

### Fixed

- Type inference error introduced by generic index in `<[T]>::get`

v0.4.0 - 2016-12-11
-------------------

### Added

- `Gamepad::mappings_source(&self)` which can be used to filter gamepads which
  not provide unified controller layout
- `MappingsSource` enum
- You can now set custom mapping for gamepad with `Gamepad::set_mapping(…)`
- `Gilrs::with_mappings(&str)` to create Gilrs with additional gamepad mappings

### Changed

- Button and axis events now also have native event codes
- On Linux, if button or axis is not known, is now reported as `Unknown`
  (previously all unknown events have been ignored)
- More devices are now treated as gamepads on Linux (use `mappings_source()` to
  filter unwanted gamepads)
- Renamed `{Gamepad,GamepadState}::is_btn_pressed(Button)` to
  `is_pressed(Button)`
- Renamed `{Gamepad,GamepadState}::axis_val(Axis)` to `value(Axis)`

### Fixed

- Integer overflow if button with keyboard code was pressed on Linux
- `Gilrs` should no longer panic if there are some unexpected problems with
  Udev
- Fixed normalization of axes values on Linux

v0.3.1 - 2016-09-23
-------------------

### Fixed

- Fixed compilation error on non-x86_64 Linux

v0.3.0 - 2016-09-22
-------------------

### Added

- `Gamepad::power_info(&self)`
- `ff::Direction::from_radians(f32)` and `ff::Direction::from_vector([f32; 2])`
- `Gilrs::gamepads(&self)` which returns iterator over all connected gamepads
- `GamepadState` now implements `is_btn_pressed(Button)` and `axis_val(Axis)`
- `Gilrs` now implements `Index`and `IndexMut`

### Changed

- Rename `Button::Unknow` to `Button::Unknown`
- `Gamepad::name(&self)` now returns `&str` instead of `&String`
- Improved dead zone detection
- `Effect::play(&self, u16)` now returns `Result<(), Error>`
- Linux: Reduced memory usage

### Removed

- `ff::Direction` no longer implements `From<f32>`

### Fixed

- Buttons west and east are no longer swapped when using SDL2 mappings
- Linux: infinite loop after gamepad disconnects
- Linux: SDL2 mappings for gamepads that can also report mouse and keyboard
  events now should works

v0.2.0 - 2016-08-18
------

### Changed

- Rename `Gilrs::pool_events()` to `Gilrs::poll_events()`

### Fixed

- Linux: Disconnected events are now emitted properly
- Linux: All force feedback effects are now dropped when gamepad disconnects
