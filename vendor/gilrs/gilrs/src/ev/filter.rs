// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Alter events in various ways.
//!
//! This modules contains "event filters" that can change, drop or create new events. To use them,
//! import `Filter` trait and call `filter()` function on `Option<Event>`. Because `filter` also
//! returns `Option<Event>` you can combine multiple filters by using `filter()` function on
//! returned event.
//!
//! Filters in this modules have public fields that can be used to configure their behaviour. You
//! can also create them with default values using `new()` method. If filter is not configurable,
//! it is implemented as function (for example `deadzone()`).
//!
//! # Example
//!
//! ```
//! use gilrs::{GilrsBuilder, Filter};
//! use gilrs::ev::filter::{Jitter, Repeat, deadzone};
//!
//! let mut gilrs = GilrsBuilder::new().with_default_filters(false).build().unwrap();
//! let jitter = Jitter { threshold: 0.02 };
//! let repeat = Repeat::new();
//!
//! // Event loop
//! loop {
//!     while let Some(event) = gilrs
//!         .next_event()
//!         .filter_ev(&jitter, &mut gilrs)
//!         .filter_ev(&deadzone, &mut gilrs)
//!         .filter_ev(&repeat, &mut gilrs)
//!     {
//!         gilrs.update(&event);
//!         println!("{:?}", event);
//!     }
//!     # break;
//! }
//! ```
//! # Implementing custom filters
//!
//! If you want to implement your own filters, you will have to implement `FilterFn` trait.
//! **Do not return `None` if you got `Some(event)`**. If you want to discard an event, uses
//! `EventType::Dropped`. Returning `None` means that there are no more events to process and
//! will end `while let` loop.
//!
//! ## Example
//!
//! Example implementations of filter that will drop all events with `Unknown` axis or button.
//!
//! ```
//! use gilrs::ev::filter::FilterFn;
//! use gilrs::{Gilrs, Event, EventType, Button, Axis, Filter};
//!
//! struct UnknownSlayer;
//!
//! impl FilterFn for UnknownSlayer {
//!     fn filter(&self, ev: Option<Event>, _gilrs: &mut Gilrs) -> Option<Event> {
//!         match ev {
//!             Some(Event { event: EventType::ButtonPressed(Button::Unknown, ..), id, .. })
//!             | Some(Event { event: EventType::ButtonReleased(Button::Unknown, ..), id, .. })
//!             | Some(Event { event: EventType::AxisChanged(Axis::Unknown, ..), id, .. })
//!             => Some(Event::new(id, EventType::Dropped)),
//!             _ => ev,
//!         }
//!     }
//! }
//! ```
//!
//! `FilterFn` is also implemented for all `Fn(Option<Event>, &Gilrs) -> Option<Event>`, so above
//! example could be simplified to passing closure to `filter()` function.

use crate::ev::{Axis, AxisOrBtn, Button, Code, Event, EventType};
use crate::gamepad::{Gamepad, Gilrs};
use crate::utils;

use std::time::Duration;

/// Discard axis events that changed less than `threshold`.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Jitter {
    pub threshold: f32,
}

impl Jitter {
    /// Creates new `Repeat` filter with threshold set to 0.01.
    pub fn new() -> Self {
        Jitter { threshold: 0.01 }
    }
}

impl Default for Jitter {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterFn for Jitter {
    fn filter(&self, ev: Option<Event>, gilrs: &mut Gilrs) -> Option<Event> {
        match ev {
            Some(Event {
                event: EventType::AxisChanged(_, val, axis),
                id,
                ..
            }) => match gilrs.gamepad(id).state().axis_data(axis) {
                Some(data) if val != 0.0 && (val - data.value()).abs() < self.threshold => {
                    Some(Event::new(id, EventType::Dropped))
                }
                _ => ev,
            },
            _ => ev,
        }
    }
}

fn apply_deadzone(x: f32, y: f32, threshold: f32) -> (f32, f32) {
    let magnitude = utils::clamp((x * x + y * y).sqrt(), 0.0, 1.0);
    if magnitude <= threshold {
        (0.0, 0.0)
    } else {
        let norm = ((magnitude - threshold) / (1.0 - threshold)) / magnitude;
        (x * norm, y * norm)
    }
}

/// Drops events in dead zone and remaps value to keep it in standard range.
pub fn deadzone(ev: Option<Event>, gilrs: &mut Gilrs) -> Option<Event> {
    match ev {
        Some(Event {
            event: EventType::AxisChanged(axis, val, nec),
            id,
            time,
        }) => {
            let threshold = match gilrs.gamepad(id).deadzone(nec) {
                Some(t) => t,
                None => return ev,
            };

            if let Some((_, other_code)) = axis
                .second_axis()
                .and_then(|axis| gilrs.gamepad(id).axis_code(axis).map(|code| (axis, code)))
            {
                let other_val = gilrs.gamepad(id).state().value(other_code);
                let val = apply_deadzone(val, other_val, threshold);

                Some(if gilrs.gamepad(id).state().value(nec) == val.0 {
                    Event::new(id, EventType::Dropped)
                } else {
                    Event {
                        id,
                        time,
                        event: EventType::AxisChanged(axis, val.0, nec),
                    }
                })
            } else {
                let val = apply_deadzone(val, 0.0, threshold).0;

                Some(if gilrs.gamepad(id).state().value(nec) == val {
                    Event::new(id, EventType::Dropped)
                } else {
                    Event {
                        id,
                        time,
                        event: EventType::AxisChanged(axis, val, nec),
                    }
                })
            }
        }
        Some(Event {
            event: EventType::ButtonChanged(btn, val, nec),
            id,
            time,
        }) => {
            let gp = &gilrs.gamepad(id);
            let threshold = match gp.deadzone(nec) {
                Some(t) => t,
                None => return ev,
            };
            let val = apply_deadzone(val, 0.0, threshold).0;

            Some(if gp.state().value(nec) == val {
                Event::new(id, EventType::Dropped)
            } else {
                Event {
                    id,
                    time,
                    event: EventType::ButtonChanged(btn, val, nec),
                }
            })
        }
        _ => ev,
    }
}

/// Maps axis dpad events to button dpad events.
///
/// This filter will do nothing if gamepad has dpad buttons (to prevent double events for same
/// element) and if standard `NativeEvCode` for dpads is used by some other buttons. It will always
/// try to map if SDL mappings contains mappings for all four hats.
pub fn axis_dpad_to_button(ev: Option<Event>, gilrs: &mut Gilrs) -> Option<Event> {
    use gilrs_core::native_ev_codes as necs;

    fn can_map(gp: &Gamepad<'_>) -> bool {
        let hats_mapped = gp.mapping().hats_mapped();
        if hats_mapped == 0b0000_1111 {
            true
        } else if hats_mapped == 0 {
            gp.axis_or_btn_name(Code(necs::BTN_DPAD_RIGHT)).is_none()
                && gp.axis_or_btn_name(Code(necs::BTN_DPAD_LEFT)).is_none()
                && gp.axis_or_btn_name(Code(necs::BTN_DPAD_DOWN)).is_none()
                && gp.axis_or_btn_name(Code(necs::BTN_DPAD_UP)).is_none()
                && gp.button_code(Button::DPadRight).is_none()
        } else {
            // Not all hats are mapped so let's ignore it for now.
            false
        }
    }

    match ev {
        Some(Event {
            event: EventType::AxisChanged(Axis::DPadX, val, _),
            id,
            time,
        }) if can_map(&gilrs.gamepad(id)) => Some(if val == 1.0 {
            gilrs.insert_event(Event {
                id,
                time,
                event: EventType::ButtonChanged(Button::DPadRight, 1.0, Code(necs::BTN_DPAD_RIGHT)),
            });
            Event {
                id,
                time,
                event: EventType::ButtonPressed(Button::DPadRight, Code(necs::BTN_DPAD_RIGHT)),
            }
        } else if val == -1.0 {
            gilrs.insert_event(Event {
                id,
                time,
                event: EventType::ButtonChanged(Button::DPadLeft, 1.0, Code(necs::BTN_DPAD_LEFT)),
            });
            Event {
                id,
                time,
                event: EventType::ButtonPressed(Button::DPadLeft, Code(necs::BTN_DPAD_LEFT)),
            }
        } else if gilrs
            .gamepad(id)
            .state()
            .is_pressed(Code(necs::BTN_DPAD_RIGHT))
        {
            gilrs.insert_event(Event {
                id,
                time,
                event: EventType::ButtonChanged(Button::DPadRight, 0.0, Code(necs::BTN_DPAD_RIGHT)),
            });
            Event {
                id,
                time,
                event: EventType::ButtonReleased(Button::DPadRight, Code(necs::BTN_DPAD_RIGHT)),
            }
        } else {
            gilrs.insert_event(Event {
                id,
                time,
                event: EventType::ButtonChanged(Button::DPadLeft, 0.0, Code(necs::BTN_DPAD_LEFT)),
            });
            Event {
                id,
                time,
                event: EventType::ButtonReleased(Button::DPadLeft, Code(necs::BTN_DPAD_LEFT)),
            }
        }),
        Some(Event {
            event: EventType::AxisChanged(Axis::DPadY, val, _),
            id,
            time,
        }) if can_map(&gilrs.gamepad(id)) => Some(if val == 1.0 {
            gilrs.insert_event(Event {
                id,
                time,
                event: EventType::ButtonChanged(Button::DPadUp, 1.0, Code(necs::BTN_DPAD_UP)),
            });
            Event {
                id,
                time,
                event: EventType::ButtonPressed(Button::DPadUp, Code(necs::BTN_DPAD_UP)),
            }
        } else if val == -1.0 {
            gilrs.insert_event(Event {
                id,
                time,
                event: EventType::ButtonChanged(Button::DPadDown, 1.0, Code(necs::BTN_DPAD_DOWN)),
            });
            Event {
                id,
                time,
                event: EventType::ButtonPressed(Button::DPadDown, Code(necs::BTN_DPAD_DOWN)),
            }
        } else if gilrs
            .gamepad(id)
            .state()
            .is_pressed(Code(necs::BTN_DPAD_UP))
        {
            gilrs.insert_event(Event {
                id,
                time,
                event: EventType::ButtonChanged(Button::DPadUp, 0.0, Code(necs::BTN_DPAD_UP)),
            });
            Event {
                id,
                time,
                event: EventType::ButtonReleased(Button::DPadUp, Code(necs::BTN_DPAD_UP)),
            }
        } else {
            gilrs.insert_event(Event {
                id,
                time,
                event: EventType::ButtonChanged(Button::DPadDown, 0.0, Code(necs::BTN_DPAD_DOWN)),
            });
            Event {
                id,
                time,
                event: EventType::ButtonReleased(Button::DPadDown, Code(necs::BTN_DPAD_DOWN)),
            }
        }),
        _ => ev,
    }
}

/// Repeats pressed keys.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Repeat {
    pub after: Duration,
    pub every: Duration,
}

impl Repeat {
    /// Creates new `Repeat` filter with `after` set to 500ms and `every` set to 30ms.
    pub fn new() -> Self {
        Repeat {
            after: Duration::from_millis(500),
            every: Duration::from_millis(30),
        }
    }
}

impl Default for Repeat {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterFn for Repeat {
    fn filter(&self, ev: Option<Event>, gilrs: &mut Gilrs) -> Option<Event> {
        match ev {
            Some(ev) => Some(ev),
            None => {
                let now = utils::time_now();
                for (id, gamepad) in gilrs.gamepads() {
                    for (nec, btn_data) in gamepad.state().buttons() {
                        match (
                            btn_data.is_pressed(),
                            btn_data.is_repeating(),
                            now.duration_since(btn_data.timestamp()),
                        ) {
                            (true, false, Ok(dur)) if dur >= self.after => {
                                let btn_name = match gamepad.axis_or_btn_name(nec) {
                                    Some(AxisOrBtn::Btn(b)) => b,
                                    _ => Button::Unknown,
                                };

                                return Some(Event {
                                    id,
                                    event: EventType::ButtonRepeated(btn_name, nec),
                                    time: btn_data.timestamp() + self.after,
                                });
                            }
                            (true, true, Ok(dur)) if dur >= self.every => {
                                let btn_name = match gamepad.axis_or_btn_name(nec) {
                                    Some(AxisOrBtn::Btn(b)) => b,
                                    _ => Button::Unknown,
                                };

                                return Some(Event {
                                    id,
                                    event: EventType::ButtonRepeated(btn_name, nec),
                                    time: btn_data.timestamp() + self.every,
                                });
                            }
                            _ => (),
                        }
                    }
                }
                None
            }
        }
    }
}

/// Allow filtering events.
///
/// See module level documentation for more info.
pub trait Filter {
    fn filter_ev<F: FilterFn>(&self, filter: &F, gilrs: &mut Gilrs) -> Option<Event>;
}

/// Actual filter implementation.
///
/// See module level documentation for more info.
pub trait FilterFn {
    fn filter(&self, ev: Option<Event>, gilrs: &mut Gilrs) -> Option<Event>;
}

impl<F> FilterFn for F
where
    F: Fn(Option<Event>, &mut Gilrs) -> Option<Event>,
{
    fn filter(&self, ev: Option<Event>, gilrs: &mut Gilrs) -> Option<Event> {
        self(ev, gilrs)
    }
}

impl Filter for Option<Event> {
    fn filter_ev<F: FilterFn>(&self, filter: &F, gilrs: &mut Gilrs) -> Option<Event> {
        let e = filter.filter(*self, gilrs);
        debug_assert!(
            !(self.is_some() && e.is_none()),
            "Filter changed Some(event) into None. See ev::filter documentation for more info."
        );

        e
    }
}

impl Filter for Event {
    fn filter_ev<F: FilterFn>(&self, filter: &F, gilrs: &mut Gilrs) -> Option<Event> {
        let e = filter.filter(Some(*self), gilrs);
        debug_assert!(
            !e.is_none(),
            "Filter changed Some(event) into None. See ev::filter documentation for more info."
        );

        e
    }
}
