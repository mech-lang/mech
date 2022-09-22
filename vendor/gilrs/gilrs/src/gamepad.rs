// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::{
    ev::{
        state::{AxisData, ButtonData, GamepadState},
        Axis, AxisOrBtn, Button, Code, Event, EventType,
    },
    ff::{
        server::{self, Message},
        Error as FfError,
    },
    mapping::{Mapping, MappingData, MappingDb},
    utils, MappingError,
};

use gilrs_core::{
    self, AxisInfo, Error as PlatformError, Event as RawEvent, EventType as RawEventType,
};

use uuid::Uuid;

use std::{
    collections::VecDeque,
    error,
    fmt::{self, Display},
    sync::mpsc::Sender,
};

pub use gilrs_core::PowerInfo;

#[cfg(feature = "serde-serialize")]
use serde::{Deserialize, Serialize};

const DEFAULT_DEADZONE: f32 = 0.1;

/// Main object responsible of managing gamepads.
///
/// In order to get gamepad handle, use `gamepad()`, or `connected_gamepad()`. The main difference
/// between these two is that `gamepad()` will also return handle to gamepad that is currently
/// disconnected. However, both functions will return `None` if gamepad with given id has never
/// existed.
///
/// # Event loop
///
/// All interesting actions like button was pressed or new controller was connected are represented
/// by struct [`Event`](struct.Event.html). Use `next_event()` function to retrieve event from
/// queue.
///
/// ```
/// use gilrs::{Gilrs, Event, EventType, Button};
///
/// let mut gilrs = Gilrs::new().unwrap();
///
/// // Event loop
/// loop {
///     while let Some(event) = gilrs.next_event() {
///         match event {
///             Event { id, event: EventType::ButtonPressed(Button::South, _), .. } => {
///                 println!("Player {}: jump!", id)
///             }
///             Event { id, event: EventType::Disconnected, .. } => {
///                 println!("We lost player {}", id)
///             }
///             _ => (),
///         };
///     }
///     # break;
/// }
/// ```
///
/// # Cached gamepad state
///
/// `Gilrs` also menage cached gamepad state. Updating state is done automatically, unless it's
///  disabled by `GilrsBuilder::set_update_state(false)`. However, if you are using custom filters,
/// you still have to update state manually – to do this call `update()` method.
///
/// To access state you can use `Gamepad::state()` function. Gamepad also implement some state
/// related functions directly, see [`Gamepad`](struct.Gamepad.html) for more.
///
/// ## Counter
///
/// `Gilrs` has additional functionality, referred here as *counter*. The idea behind it is simple,
/// each time you end iteration of update loop, you call `Gilrs::inc()` which will increase
/// internal counter by one. When state of one if elements changes, value of counter is saved. When
/// checking state of one of elements you can tell exactly when this event happened. Timestamps are
/// not good solution here because they can tell you when *system* observed event, not when you
/// processed it. On the other hand, they are good when you want to implement key repeat or software
/// debouncing.
///
/// ```
/// use gilrs::{Gilrs, Button};
///
/// let mut gilrs = Gilrs::new().unwrap();
/// let mut player_one = None;
///
/// loop {
///     while let Some(ev) = gilrs.next_event() {
///         if player_one.is_none() {
///             player_one = Some(ev.id);
///         }
///
///         // Do other things with event
///     }
///
///     if let Some(id) = player_one {
///         let gamepad = gilrs.gamepad(id);
///
///         if gamepad.is_pressed(Button::DPadLeft) {
///             // go left
///         }
///
///         match gamepad.button_data(Button::South) {
///             Some(d) if d.is_pressed() && d.counter() == gilrs.counter() => {
///                 // jump only if button was observed to be pressed in this iteration
///             }
///             _ => ()
///         }
///     }
///
///     // Increase counter
///     gilrs.inc();
/// #   break;
/// }
///
#[derive(Debug)]
pub struct Gilrs {
    inner: gilrs_core::Gilrs,
    next_id: usize,
    tx: Sender<Message>,
    counter: u64,
    mappings: MappingDb,
    default_filters: bool,
    events: VecDeque<Event>,
    axis_to_btn_pressed: f32,
    axis_to_btn_released: f32,
    update_state: bool,
    gamepads_data: Vec<GamepadData>,
}

impl Gilrs {
    /// Creates new `Gilrs` with default settings. See [`GilrsBuilder`](struct.GilrsBuilder.html)
    /// for more details.
    pub fn new() -> Result<Self, Error> {
        GilrsBuilder::new().build()
    }

    /// Returns next pending event. If there is no pending event, `None` is
    /// returned. This function will not block current thread and should be safe
    /// to call in async context.
    pub fn next_event(&mut self) -> Option<Event> {
        use crate::ev::filter::{axis_dpad_to_button, deadzone, Filter, Jitter};

        let ev = if self.default_filters {
            let jitter_filter = Jitter::new();
            loop {
                let ev = self
                    .next_event_priv()
                    .filter_ev(&axis_dpad_to_button, self)
                    .filter_ev(&jitter_filter, self)
                    .filter_ev(&deadzone, self);

                // Skip all dropped events, there is no reason to return them
                match ev {
                    Some(ev) if ev.is_dropped() => (),
                    _ => break ev,
                }
            }
        } else {
            self.next_event_priv()
        };

        if self.update_state {
            if let Some(ref ev) = ev {
                self.update(ev);
            }
        }

        ev
    }

    /// Returns next pending event.
    fn next_event_priv(&mut self) -> Option<Event> {
        if let Some(ev) = self.events.pop_front() {
            Some(ev)
        } else {
            match self.inner.next_event() {
                Some(RawEvent { id, event, time }) => {
                    trace!("Original event: {:?}", RawEvent { id, event, time });
                    let id = GamepadId(id);

                    let event = match event {
                        RawEventType::ButtonPressed(nec) => {
                            let nec = Code(nec);
                            match self.gamepad(id).axis_or_btn_name(nec) {
                                Some(AxisOrBtn::Btn(b)) => {
                                    self.events.push_back(Event {
                                        id,
                                        time,
                                        event: EventType::ButtonChanged(b, 1.0, nec),
                                    });

                                    EventType::ButtonPressed(b, nec)
                                }
                                Some(AxisOrBtn::Axis(a)) => EventType::AxisChanged(a, 1.0, nec),
                                None => {
                                    self.events.push_back(Event {
                                        id,
                                        time,
                                        event: EventType::ButtonChanged(Button::Unknown, 1.0, nec),
                                    });

                                    EventType::ButtonPressed(Button::Unknown, nec)
                                }
                            }
                        }
                        RawEventType::ButtonReleased(nec) => {
                            let nec = Code(nec);
                            match self.gamepad(id).axis_or_btn_name(nec) {
                                Some(AxisOrBtn::Btn(b)) => {
                                    self.events.push_back(Event {
                                        id,
                                        time,
                                        event: EventType::ButtonChanged(b, 0.0, nec),
                                    });

                                    EventType::ButtonReleased(b, nec)
                                }
                                Some(AxisOrBtn::Axis(a)) => EventType::AxisChanged(a, 0.0, nec),
                                None => {
                                    self.events.push_back(Event {
                                        id,
                                        time,
                                        event: EventType::ButtonChanged(Button::Unknown, 0.0, nec),
                                    });

                                    EventType::ButtonReleased(Button::Unknown, nec)
                                }
                            }
                        }
                        RawEventType::AxisValueChanged(val, nec) => {
                            // Let's trust at least our backend code
                            let axis_info = *self.gamepad(id).inner.axis_info(nec).unwrap();
                            let nec = Code(nec);

                            match self.gamepad(id).axis_or_btn_name(nec) {
                                Some(AxisOrBtn::Btn(b)) => {
                                    let val = btn_value(&axis_info, val);

                                    if val >= self.axis_to_btn_pressed
                                        && !self.gamepad(id).state().is_pressed(nec)
                                    {
                                        self.events.push_back(Event {
                                            id,
                                            time,
                                            event: EventType::ButtonChanged(b, val, nec),
                                        });

                                        EventType::ButtonPressed(b, nec)
                                    } else if val <= self.axis_to_btn_released
                                        && self.gamepad(id).state().is_pressed(nec)
                                    {
                                        self.events.push_back(Event {
                                            id,
                                            time,
                                            event: EventType::ButtonChanged(b, val, nec),
                                        });

                                        EventType::ButtonReleased(b, nec)
                                    } else {
                                        EventType::ButtonChanged(b, val, nec)
                                    }
                                }
                                Some(AxisOrBtn::Axis(a)) => {
                                    EventType::AxisChanged(a, axis_value(&axis_info, val, a), nec)
                                }
                                None => EventType::AxisChanged(
                                    Axis::Unknown,
                                    axis_value(&axis_info, val, Axis::Unknown),
                                    nec,
                                ),
                            }
                        }
                        RawEventType::Connected => {
                            if id.0 == self.gamepads_data.len() {
                                self.gamepads_data.push(GamepadData::new(
                                    id,
                                    self.tx.clone(),
                                    self.inner.gamepad(id.0).unwrap(),
                                    &self.mappings,
                                ));
                            } else if id.0 < self.gamepads_data.len() {
                                self.gamepads_data[id.0] = GamepadData::new(
                                    id,
                                    self.tx.clone(),
                                    self.inner.gamepad(id.0).unwrap(),
                                    &self.mappings,
                                );
                            } else {
                                error!(
                                    "Platform implementation error: got Connected event with id \
                                     {}, when expected id {}",
                                    id.0,
                                    self.gamepads_data.len()
                                );
                            }

                            EventType::Connected
                        }
                        RawEventType::Disconnected => {
                            let _ = self.tx.send(Message::Close { id: id.0 });

                            EventType::Disconnected
                        }
                    };

                    Some(Event { id, event, time })
                }
                None => None,
            }
        }
    }

    /// Updates internal state according to `event`.
    ///
    /// Please note, that it's not necessary to call this function unless you modify events by using
    /// additional filters and disabled automatic updates when creating `Gilrs`.
    pub fn update(&mut self, event: &Event) {
        use crate::EventType::*;

        let counter = self.counter;

        let data = match self.gamepads_data.get_mut(event.id.0) {
            Some(d) => d,
            None => return,
        };

        match event.event {
            ButtonPressed(_, nec) => {
                data.state.set_btn_pressed(nec, true, counter, event.time);
            }
            ButtonReleased(_, nec) => {
                data.state.set_btn_pressed(nec, false, counter, event.time);
            }
            ButtonRepeated(_, nec) => {
                data.state.set_btn_repeating(nec, counter, event.time);
            }
            ButtonChanged(_, value, nec) => {
                data.state.set_btn_value(nec, value, counter, event.time);
            }
            AxisChanged(_, value, nec) => {
                data.state
                    .update_axis(nec, AxisData::new(value, counter, event.time));
            }
            Disconnected | Connected | Dropped => (),
        }
    }

    /// Increases internal counter by one. Counter data is stored with state and can be used to
    /// determine when last event happened. You probably want to use this function in your update
    /// loop after processing events.
    pub fn inc(&mut self) {
        // Counter is 62bit. See `ButtonData`.
        if self.counter == 0x3FFF_FFFF_FFFF_FFFF {
            self.counter = 0;
        } else {
            self.counter += 1;
        }
    }

    /// Returns counter. Counter data is stored with state and can be used to determine when last
    /// event happened.
    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// Sets counter to 0.
    pub fn reset_counter(&mut self) {
        self.counter = 0;
    }

    fn finish_gamepads_creation(&mut self) {
        let tx = self.tx.clone();
        for id in 0..self.inner.last_gamepad_hint() {
            let gamepad = self.inner.gamepad(id).unwrap();
            self.gamepads_data.push(GamepadData::new(
                GamepadId(id),
                tx.clone(),
                gamepad,
                &self.mappings,
            ))
        }
    }

    /// Returns handle to gamepad with given ID. Unlike `connected_gamepad()`, this function will
    /// also return handle to gamepad that is currently disconnected.
    ///
    /// ```
    /// # let mut gilrs = gilrs::Gilrs::new().unwrap();
    /// use gilrs::{Button, EventType};
    ///
    /// loop {
    ///     while let Some(ev) = gilrs.next_event() {
    ///         // unwrap() should never panic because we use id from event
    ///         let is_up_pressed = gilrs.gamepad(ev.id).is_pressed(Button::DPadUp);
    ///
    ///         match ev.event {
    ///             EventType::ButtonPressed(Button::South, _) if is_up_pressed => {
    ///                 // do something…
    ///             }
    ///             _ => (),
    ///         }
    ///     }
    ///     # break;
    /// }
    /// ```
    pub fn gamepad(&self, id: GamepadId) -> Gamepad {
        Gamepad {
            inner: self.inner.gamepad(id.0).unwrap(),
            data: &self.gamepads_data[id.0],
        }
    }

    /// Returns a reference to connected gamepad or `None`.
    pub fn connected_gamepad(&self, id: GamepadId) -> Option<Gamepad<'_>> {
        // Make sure that it will not panic even with invalid GamepadId, so ConnectedGamepadIterator
        // will always work.
        if let Some(data) = self.gamepads_data.get(id.0) {
            let inner = self.inner.gamepad(id.0).unwrap();

            if inner.is_connected() {
                Some(Gamepad { inner, data })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Returns iterator over all connected gamepads and their ids.
    ///
    /// ```
    /// # let gilrs = gilrs::Gilrs::new().unwrap();
    /// for (id, gamepad) in gilrs.gamepads() {
    ///     assert!(gamepad.is_connected());
    ///     println!("Gamepad with id {} and name {} is connected",
    ///              id, gamepad.name());
    /// }
    /// ```
    pub fn gamepads(&self) -> ConnectedGamepadsIterator<'_> {
        ConnectedGamepadsIterator(self, 0)
    }

    /// Adds `ev` at the end of internal event queue. It can later be retrieved with `next_event()`.
    pub fn insert_event(&mut self, ev: Event) {
        self.events.push_back(ev);
    }

    pub(crate) fn ff_sender(&self) -> &Sender<Message> {
        &self.tx
    }

    /// Sets gamepad's mapping and returns SDL2 representation of them. Returned mappings may not be
    /// compatible with SDL2 - if it is important, use
    /// [`set_mapping_strict()`](#method.set_mapping_strict).
    ///
    /// The `name` argument can be a string slice with custom gamepad name or `None`. If `None`,
    /// gamepad name reported by driver will be used.
    ///
    /// # Errors
    ///
    /// This function return error if `name` contains comma, `mapping` have axis and button entry
    /// for same element (for example `Axis::LetfTrigger` and `Button::LeftTrigger`) or gamepad does
    /// not have any element with `EvCode` used in mapping. `Button::Unknown` and
    /// `Axis::Unknown` are not allowd as keys to `mapping` – in this case,
    /// `MappingError::UnknownElement` is returned.
    ///
    /// Error is also returned if this function is not implemented or gamepad is not connected.
    ///
    /// # Example
    ///
    /// ```
    /// use gilrs::{Mapping, Button};
    ///
    /// # let mut gilrs = gilrs::Gilrs::new().unwrap();
    /// let mut data = Mapping::new();
    /// // …
    ///
    /// // or `match gilrs.set_mapping(0, &data, None) {`
    /// match gilrs.set_mapping(0, &data, "Custom name") {
    ///     Ok(sdl) => println!("SDL2 mapping: {}", sdl),
    ///     Err(e) => println!("Failed to set mapping: {}", e),
    /// };
    /// ```
    ///
    /// See also `examples/mapping.rs`.
    pub fn set_mapping<'b, O: Into<Option<&'b str>>>(
        &mut self,
        gamepad_id: usize,
        mapping: &MappingData,
        name: O,
    ) -> Result<String, MappingError> {
        if let Some(gamepad) = self.inner.gamepad(gamepad_id) {
            if gamepad.is_connected() {
                return Err(MappingError::NotConnected);
            }

            let name = match name.into() {
                Some(s) => s,
                None => gamepad.name(),
            };

            let (mapping, s) = Mapping::from_data(
                mapping,
                gamepad.buttons(),
                gamepad.axes(),
                name,
                Uuid::from_bytes(gamepad.uuid()),
            )?;

            // We checked if gamepad is connected, so it should never panic
            let data = &mut self.gamepads_data[gamepad_id];
            data.mapping = mapping;

            Ok(s)
        } else {
            Err(MappingError::NotConnected)
        }
    }

    /// Similar to [`set_mapping()`](#method.set_mapping) but returned string should be compatible
    /// with SDL2.
    ///
    /// # Errors
    ///
    /// Returns `MappingError::NotSdl2Compatible` if `mapping` have an entry for `Button::{C, Z}`
    /// or `Axis::{LeftZ, RightZ}`.
    pub fn set_mapping_strict<'b, O: Into<Option<&'b str>>>(
        &mut self,
        gamepad_id: usize,
        mapping: &MappingData,
        name: O,
    ) -> Result<String, MappingError> {
        if mapping.button(Button::C).is_some()
            || mapping.button(Button::Z).is_some()
            || mapping.axis(Axis::LeftZ).is_some()
            || mapping.axis(Axis::RightZ).is_some()
        {
            Err(MappingError::NotSdl2Compatible)
        } else {
            self.set_mapping(gamepad_id, mapping, name)
        }
    }

    pub(crate) fn next_ff_id(&mut self) -> usize {
        // TODO: reuse free ids
        let id = self.next_id;
        self.next_id = match self.next_id.checked_add(1) {
            Some(x) => x,
            None => panic!("Failed to assign ID to new effect"),
        };
        id
    }
}

/// Allow to create `Gilrs ` with customized behaviour.
pub struct GilrsBuilder {
    mappings: MappingDb,
    default_filters: bool,
    axis_to_btn_pressed: f32,
    axis_to_btn_released: f32,
    update_state: bool,
    env_mappings: bool,
    included_mappings: bool,
}

impl GilrsBuilder {
    /// Create builder with default settings. Use `build()` to create `Gilrs`.
    pub fn new() -> Self {
        GilrsBuilder {
            mappings: MappingDb::new(),
            default_filters: true,
            axis_to_btn_pressed: 0.75,
            axis_to_btn_released: 0.65,
            update_state: true,
            env_mappings: true,
            included_mappings: true,
        }
    }

    /// If `true`, use [`axis_dpad_to_button`](ev/filter/fn.axis_dpad_to_button.html),
    /// [`Jitter`](ev/filter/struct.Jitter.html) and [`deadzone`](ev/filter/fn.deadzone.html)
    /// filters with default parameters. Defaults to `true`.
    pub fn with_default_filters(mut self, default_filters: bool) -> Self {
        self.default_filters = default_filters;

        self
    }

    /// Adds SDL mappings.
    pub fn add_mappings(mut self, mappings: &str) -> Self {
        self.mappings.insert(mappings);

        self
    }

    /// If true, will add SDL mappings from `SDL_GAMECONTROLLERCONFIG` environment variable.
    /// Defaults to true.
    pub fn add_env_mappings(mut self, env_mappings: bool) -> Self {
        self.env_mappings = env_mappings;

        self
    }

    /// If true, will add SDL mappings included from
    /// https://github.com/gabomdq/SDL_GameControllerDB. Defaults to true.
    pub fn add_included_mappings(mut self, included_mappings: bool) -> Self {
        self.included_mappings = included_mappings;

        self
    }

    /// Sets values on which `ButtonPressed` and `ButtonReleased` events will be emitted. `build()`
    /// will return error if `pressed ≤ released` or if one of values is outside [0.0, 1.0].
    ///
    /// Defaults to 0.75 for `pressed` and 0.65 for `released`.
    pub fn set_axis_to_btn(mut self, pressed: f32, released: f32) -> Self {
        self.axis_to_btn_pressed = pressed;
        self.axis_to_btn_released = released;

        self
    }

    /// Disable or enable automatic state updates. You should use this if you use custom filters;
    /// in this case you have to update state manually anyway.
    pub fn set_update_state(mut self, enabled: bool) -> Self {
        self.update_state = enabled;

        self
    }

    /// Creates `Gilrs`.
    pub fn build(mut self) -> Result<Gilrs, Error> {
        if self.included_mappings {
            self.mappings.add_included_mappings();
        }

        if self.env_mappings {
            self.mappings.add_env_mappings();
        }

        debug!("Loaded {} mappings.", self.mappings.len());

        if self.axis_to_btn_pressed <= self.axis_to_btn_released
            || self.axis_to_btn_pressed < 0.0
            || self.axis_to_btn_pressed > 1.0
            || self.axis_to_btn_released < 0.0
            || self.axis_to_btn_released > 1.0
        {
            return Err(Error::InvalidAxisToBtn);
        }

        let mut is_dummy = false;
        let inner = match gilrs_core::Gilrs::new() {
            Ok(g) => g,
            Err(PlatformError::NotImplemented(g)) => {
                is_dummy = true;

                g
            }
            Err(PlatformError::Other(e)) => return Err(Error::Other(e)),
        };

        let mut gilrs = Gilrs {
            inner,
            next_id: 0,
            tx: server::init(),
            counter: 0,
            mappings: self.mappings,
            default_filters: self.default_filters,
            events: VecDeque::new(),
            axis_to_btn_pressed: self.axis_to_btn_pressed,
            axis_to_btn_released: self.axis_to_btn_released,
            update_state: self.update_state,
            gamepads_data: Vec::new(),
        };
        gilrs.finish_gamepads_creation();

        if is_dummy {
            Err(Error::NotImplemented(gilrs))
        } else {
            Ok(gilrs)
        }
    }
}

impl Default for GilrsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over all connected gamepads.
pub struct ConnectedGamepadsIterator<'a>(&'a Gilrs, usize);

impl<'a> Iterator for ConnectedGamepadsIterator<'a> {
    type Item = (GamepadId, Gamepad<'a>);

    fn next(&mut self) -> Option<(GamepadId, Gamepad<'a>)> {
        loop {
            if self.1 == self.0.inner.last_gamepad_hint() {
                return None;
            }

            if let Some(gp) = self.0.connected_gamepad(GamepadId(self.1)) {
                let idx = self.1;
                self.1 += 1;
                return Some((GamepadId(idx), gp));
            }

            self.1 += 1;
        }
    }
}

/// Represents handle to game controller.
///
/// Using this struct you can access cached gamepad state, information about gamepad such as name
/// or UUID and manage force feedback effects.
#[derive(Debug, Copy, Clone)]
pub struct Gamepad<'a> {
    data: &'a GamepadData,
    inner: &'a gilrs_core::Gamepad,
}

impl<'a> Gamepad<'a> {
    /// Returns the mapping name if it exists otherwise returns the os provided name.
    pub fn name(&self) -> &str {
        if let Some(map_name) = self.map_name() {
            map_name
        } else {
            self.os_name()
        }
    }

    /// if `mapping_source()` is `SdlMappings` returns the name of the mapping used by the gamepad.
    /// Otherwise returns `None`.
    pub fn map_name(&self) -> Option<&str> {
        self.data.map_name()
    }

    /// Returns the name of the gamepad supplied by the OS.
    pub fn os_name(&self) -> &str {
        self.inner.name()
    }

    /// Returns gamepad's UUID.
    ///
    /// It is recommended to process with the [UUID crate](https://crates.io/crates/uuid).
    /// Use `Uuid::from_bytes` method to create a `Uuid` from the returned bytes.
    pub fn uuid(&self) -> [u8; 16] {
        self.inner.uuid()
    }

    /// Returns cached gamepad state.
    pub fn state(&self) -> &GamepadState {
        &self.data.state
    }

    /// Returns true if gamepad is connected.
    pub fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    /// Examines cached gamepad state to check if given button is pressed. Panics if `btn` is
    /// `Unknown`.
    ///
    /// If you know `Code` of the element that you want to examine, it's recommended to use methods
    /// directly on `State`, because this version have to check which `Code` is mapped to element of
    /// gamepad.
    pub fn is_pressed(&self, btn: Button) -> bool {
        self.data.is_pressed(btn)
    }

    /// Examines cached gamepad state to check axis's value. Panics if `axis` is `Unknown`.
    ///
    /// If you know `Code` of the element that you want to examine, it's recommended to use methods
    /// directly on `State`, because this version have to check which `Code` is mapped to element of
    /// gamepad.
    pub fn value(&self, axis: Axis) -> f32 {
        self.data.value(axis)
    }

    /// Returns button state and when it changed.
    ///
    /// If you know `Code` of the element that you want to examine, it's recommended to use methods
    /// directly on `State`, because this version have to check which `Code` is mapped to element of
    /// gamepad.
    pub fn button_data(&self, btn: Button) -> Option<&ButtonData> {
        self.data.button_data(btn)
    }

    /// Returns axis state and when it changed.
    ///
    /// If you know `Code` of the element that you want to examine, it's recommended to use methods
    /// directly on `State`, because this version have to check which `Code` is mapped to element of
    /// gamepad.
    pub fn axis_data(&self, axis: Axis) -> Option<&AxisData> {
        self.data.axis_data(axis)
    }

    /// Returns device's power supply state. See [`PowerInfo`](enum.PowerInfo.html) for details.
    pub fn power_info(&self) -> PowerInfo {
        self.inner.power_info()
    }

    /// Returns source of gamepad mapping. Can be used to filter gamepads which do not provide
    /// unified controller layout.
    ///
    /// ```
    /// use gilrs::MappingSource;
    /// # let mut gilrs = gilrs::Gilrs::new().unwrap();
    ///
    /// for (_, gamepad) in gilrs.gamepads().filter(
    ///     |gp| gp.1.mapping_source() != MappingSource::None)
    /// {
    ///     println!("{} is ready to use!", gamepad.name());
    /// }
    /// ```
    pub fn mapping_source(&self) -> MappingSource {
        if self.data.mapping.is_default() {
            // TODO: check if it's Driver or None
            MappingSource::Driver
        } else {
            MappingSource::SdlMappings
        }
    }

    /// Returns true if force feedback is supported by device.
    pub fn is_ff_supported(&self) -> bool {
        self.inner.is_ff_supported()
    }

    /// Change gamepad position used by force feedback effects.
    pub fn set_listener_position<Vec3: Into<[f32; 3]>>(
        &self,
        position: Vec3,
    ) -> Result<(), FfError> {
        if !self.is_connected() {
            Err(FfError::Disconnected(self.id()))
        } else if !self.is_ff_supported() {
            Err(FfError::FfNotSupported(self.id()))
        } else {
            self.data.tx.send(Message::SetListenerPosition {
                id: self.data.id.0,
                position: position.into(),
            })?;
            Ok(())
        }
    }

    /// Returns `AxisOrBtn` mapped to `Code`.
    pub fn axis_or_btn_name(&self, ec: Code) -> Option<AxisOrBtn> {
        self.data.axis_or_btn_name(ec)
    }

    /// Returns `Code` associated with `btn`.
    pub fn button_code(&self, btn: Button) -> Option<Code> {
        self.data.button_code(btn)
    }

    /// Returns `Code` associated with `axis`.
    pub fn axis_code(&self, axis: Axis) -> Option<Code> {
        self.data.axis_code(axis)
    }

    /// Returns area in which axis events should be ignored.
    pub fn deadzone(&self, axis: Code) -> Option<f32> {
        self.inner.axis_info(axis.0).map(|i| {
            let range = i.max as f32 - i.min as f32;

            if range == 0.0 {
                0.0
            } else {
                i.deadzone
                    .map(|d| d as f32 / range * 2.0)
                    .unwrap_or(DEFAULT_DEADZONE)
            }
        })
    }

    /// Returns ID of gamepad.
    pub fn id(&self) -> GamepadId {
        self.data.id
    }

    pub(crate) fn mapping(&self) -> &Mapping {
        &self.data.mapping
    }
}

#[derive(Debug)]
struct GamepadData {
    state: GamepadState,
    mapping: Mapping,
    tx: Sender<Message>,
    id: GamepadId,
}

impl GamepadData {
    fn new(
        id: GamepadId,
        tx: Sender<Message>,
        gamepad: &gilrs_core::Gamepad,
        db: &MappingDb,
    ) -> Self {
        let mapping = db
            .get(Uuid::from_bytes(gamepad.uuid()))
            .and_then(|s| Mapping::parse_sdl_mapping(s, gamepad.buttons(), gamepad.axes()).ok())
            .unwrap_or_else(|| Mapping::default(gamepad));

        if gamepad.is_ff_supported() && gamepad.is_connected() {
            if let Some(device) = gamepad.ff_device() {
                let _ = tx.send(Message::Open { id: id.0, device });
            }
        }

        GamepadData {
            state: GamepadState::new(),
            mapping,
            tx,
            id,
        }
    }

    /// if `mapping_source()` is `SdlMappings` returns the name of the mapping used by the gamepad.
    /// Otherwise returns `None`.
    ///
    /// Warning: Mappings are set after event `Connected` is processed therefore this function will
    /// always return `None` before first calls to `Gilrs::next_event()`.
    pub fn map_name(&self) -> Option<&str> {
        if self.mapping.is_default() {
            None
        } else {
            Some(&self.mapping.name())
        }
    }

    /// Examines cached gamepad state to check if given button is pressed. Panics if `btn` is
    /// `Unknown`.
    ///
    /// If you know `Code` of the element that you want to examine, it's recommended to use methods
    /// directly on `State`, because this version have to check which `Code` is mapped to element of
    /// gamepad.
    pub fn is_pressed(&self, btn: Button) -> bool {
        assert_ne!(btn, Button::Unknown);

        self.button_code(btn)
            .or_else(|| btn.to_nec())
            .map(|nec| self.state.is_pressed(nec))
            .unwrap_or(false)
    }

    /// Examines cached gamepad state to check axis's value. Panics if `axis` is `Unknown`.
    ///
    /// If you know `Code` of the element that you want to examine, it's recommended to use methods
    /// directly on `State`, because this version have to check which `Code` is mapped to element of
    /// gamepad.
    pub fn value(&self, axis: Axis) -> f32 {
        assert_ne!(axis, Axis::Unknown);

        self.axis_code(axis)
            .map(|nec| self.state.value(nec))
            .unwrap_or(0.0)
    }

    /// Returns button state and when it changed.
    ///
    /// If you know `Code` of the element that you want to examine, it's recommended to use methods
    /// directly on `State`, because this version have to check which `Code` is mapped to element of
    /// gamepad.
    pub fn button_data(&self, btn: Button) -> Option<&ButtonData> {
        self.button_code(btn)
            .and_then(|nec| self.state.button_data(nec))
    }

    /// Returns axis state and when it changed.
    ///
    /// If you know `Code` of the element that you want to examine, it's recommended to use methods
    /// directly on `State`, because this version have to check which `Code` is mapped to element of
    /// gamepad.
    pub fn axis_data(&self, axis: Axis) -> Option<&AxisData> {
        self.axis_code(axis)
            .and_then(|nec| self.state.axis_data(nec))
    }

    /// Returns `AxisOrBtn` mapped to `Code`.
    pub fn axis_or_btn_name(&self, ec: Code) -> Option<AxisOrBtn> {
        self.mapping.map(&ec.0)
    }

    /// Returns `Code` associated with `btn`.
    pub fn button_code(&self, btn: Button) -> Option<Code> {
        self.mapping.map_rev(&AxisOrBtn::Btn(btn)).map(Code)
    }

    /// Returns `Code` associated with `axis`.
    pub fn axis_code(&self, axis: Axis) -> Option<Code> {
        self.mapping.map_rev(&AxisOrBtn::Axis(axis)).map(Code)
    }
}

/// Source of gamepad mappings.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MappingSource {
    /// Gamepad uses SDL mappings.
    SdlMappings,
    /// Gamepad does not use any mappings but driver should provide unified controller layout.
    Driver,
    /// Gamepad does not use any mappings and most gamepad events will probably be `Button::Unknown`
    /// or `Axis::Unknown`
    None,
}

/// Gamepad ID.
///
/// It's not possible to create instance of this type directly, but you can obtain one from Gamepad
/// handle or any event. ID is valid for entire lifetime of `Gilrs` context.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
#[cfg_attr(feature = "serde-serialize", derive(Serialize, Deserialize))]
pub struct GamepadId(pub(crate) usize);

impl Into<usize> for GamepadId {
    fn into(self) -> usize {
        self.0
    }
}

impl Display for GamepadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

fn axis_value(info: &AxisInfo, val: i32, axis: Axis) -> f32 {
    let mut range = info.max as f32 - info.min as f32;
    let mut val = val as f32 - info.min as f32;

    if let Some(i_range) = info.max.checked_sub(info.min) {
        // Only consider adjusting range & val if calculating the range doesn't cause overflow.  If
        // the range is so large overflow occurs, adjusting values by 1.0 would be insignificant.
        if i_range % 2 == 1 {
            // Add one to range and val, so value at center (like 127/255) will be mapped 0.0
            range += 1.0;
            val += 1.0;
        }
    }

    val = val / range * 2.0 - 1.0;

    if gilrs_core::IS_Y_AXIS_REVERSED
        && (axis == Axis::LeftStickY || axis == Axis::RightStickY || axis == Axis::DPadY)
        && val != 0.0
    {
        val = -val;
    }

    utils::clamp(val, -1.0, 1.0)
}

fn btn_value(info: &AxisInfo, val: i32) -> f32 {
    let range = (info.max - info.min) as f32;
    let mut val = (val - info.min) as f32;
    val /= range;

    utils::clamp(val, 0.0, 1.0)
}

/// Error type which can be returned when creating `Gilrs`.
#[derive(Debug)]
pub enum Error {
    /// Gilrs does not support current platform, but you can use dummy context from this error if
    /// gamepad input is not essential.
    NotImplemented(Gilrs),
    /// Either `pressed ≤ released` or one of values is outside [0.0, 1.0] range.
    InvalidAxisToBtn,
    /// Platform specific error.
    Other(Box<dyn error::Error + Send + Sync + 'static>),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotImplemented(_) => f.write_str("Gilrs does not support current platform."),
            Error::InvalidAxisToBtn => f.write_str(
                "Either `pressed ≤ released` or one of values is outside [0.0, 1.0] range.",
            ),
            Error::Other(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Other(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{axis_value, Axis, AxisInfo};

    #[test]
    fn axis_value_documented_case() {
        let info = AxisInfo {
            min: 0,
            max: 255,
            deadzone: None,
        };
        let axis = Axis::LeftStickY;
        assert_eq!(0., axis_value(&info, 127, axis));
    }
    #[test]
    fn axis_value_overflow() {
        let info = AxisInfo {
            min: std::i32::MIN,
            max: std::i32::MAX,
            deadzone: None,
        };
        let axis = Axis::LeftStickY;

        assert_eq!(0., axis_value(&info, -1, axis));
        assert_eq!(0., axis_value(&info, 0, axis));
        assert_eq!(0., axis_value(&info, 1, axis));
    }
}
