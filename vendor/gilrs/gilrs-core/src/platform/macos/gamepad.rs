// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::io_kit::*;
use super::FfDevice;
use crate::{AxisInfo, Event, EventType, PlatformError, PowerInfo};
use uuid::Uuid;

use core_foundation::runloop::{kCFRunLoopDefaultMode, CFRunLoop};
use io_kit_sys::hid::base::{IOHIDDeviceRef, IOHIDValueRef};
use io_kit_sys::hid::usage_tables::{
    kHIDPage_GenericDesktop, kHIDUsage_GD_GamePad, kHIDUsage_GD_Joystick,
    kHIDUsage_GD_MultiAxisController,
};
use io_kit_sys::ret::IOReturn;
use vec_map::VecMap;

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::os::raw::c_void;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug)]
pub struct Gilrs {
    gamepads: Vec<Gamepad>,
    device_infos: Arc<Mutex<Vec<DeviceInfo>>>,
    rx: Receiver<(Event, Option<IOHIDDevice>)>,
}

impl Gilrs {
    pub(crate) fn new() -> Result<Self, PlatformError> {
        let gamepads = Vec::new();
        let device_infos = Arc::new(Mutex::new(Vec::new()));

        let (tx, rx) = mpsc::channel();
        Self::spawn_thread(tx, device_infos.clone());

        Ok(Gilrs {
            gamepads,
            device_infos,
            rx,
        })
    }

    fn spawn_thread(
        tx: Sender<(Event, Option<IOHIDDevice>)>,
        device_infos: Arc<Mutex<Vec<DeviceInfo>>>,
    ) {
        thread::spawn(move || unsafe {
            let mut manager = match IOHIDManager::new() {
                Some(manager) => manager,
                None => {
                    error!("Failed to create IOHIDManager object");
                    return;
                }
            };

            manager.schedule_with_run_loop(CFRunLoop::get_current(), kCFRunLoopDefaultMode);

            let context = &(tx.clone(), device_infos.clone()) as *const _ as *mut c_void;
            manager.register_device_matching_callback(device_matching_cb, context);

            let context = &(tx.clone(), device_infos.clone()) as *const _ as *mut c_void;
            manager.register_device_removal_callback(device_removal_cb, context);

            let context = &(tx, device_infos) as *const _ as *mut c_void;
            manager.register_input_value_callback(input_value_cb, context);

            CFRunLoop::run_current();

            manager.unschedule_from_run_loop(CFRunLoop::get_current(), kCFRunLoopDefaultMode);
        });
    }

    pub(crate) fn next_event(&mut self) -> Option<Event> {
        match self.rx.try_recv().ok() {
            Some((event, Some(device))) => {
                if event.event == EventType::Connected {
                    if self.gamepads.get(event.id).is_some() {
                        self.gamepads[event.id].is_connected = true;
                    } else {
                        match Gamepad::open(device) {
                            Some(gamepad) => {
                                self.gamepads.push(gamepad);
                            }
                            None => {
                                error!("Failed to open gamepad: {:?}", event.id);
                                return None;
                            }
                        };
                    }
                }
                Some(event)
            }
            Some((event, None)) => {
                if event.event == EventType::Disconnected {
                    match self.gamepads.get_mut(event.id) {
                        Some(gamepad) => {
                            match self.device_infos.lock().unwrap().get_mut(event.id) {
                                Some(device_info) => device_info.is_connected = false,
                                None => {
                                    error!("Failed to find device_info: {:?}", event.id);
                                    return None;
                                }
                            };
                            gamepad.is_connected = false;
                        }
                        None => {
                            error!("Failed to find gamepad: {:?}", event.id);
                            return None;
                        }
                    }
                }
                Some(event)
            }
            None => None,
        }
    }

    pub fn gamepad(&self, id: usize) -> Option<&Gamepad> {
        self.gamepads.get(id)
    }

    /// Returns index greater than index of last connected gamepad.
    pub fn last_gamepad_hint(&self) -> usize {
        self.gamepads.len()
    }
}

#[derive(Debug)]
pub struct Gamepad {
    name: String,
    uuid: Uuid,
    entry_id: u64,
    location_id: u32,
    page: u32,
    usage: u32,
    axes_info: VecMap<AxisInfo>,
    axes: Vec<EvCode>,
    buttons: Vec<EvCode>,
    is_connected: bool,
}

impl Gamepad {
    fn open(device: IOHIDDevice) -> Option<Gamepad> {
        let io_service = match device.get_service() {
            Some(io_service) => io_service,
            None => {
                error!("Failed to get device service");
                return None;
            }
        };

        let entry_id = match io_service.get_registry_entry_id() {
            Some(entry_id) => entry_id,
            None => {
                error!("Failed to get entry id of device");
                return None;
            }
        };

        let location_id = match device.get_location_id() {
            Some(location_id) => location_id,
            None => {
                error!("Failed to get location id of device");
                return None;
            }
        };

        let page = match device.get_page() {
            Some(page) => {
                if page == kHIDPage_GenericDesktop {
                    page
                } else {
                    error!("Failed to get valid device: {:?}", page);
                    return None;
                }
            }
            None => {
                error!("Failed to get page of device");
                return None;
            }
        };

        let usage = match device.get_usage() {
            Some(usage) => {
                if usage == kHIDUsage_GD_GamePad
                    || usage == kHIDUsage_GD_Joystick
                    || usage == kHIDUsage_GD_MultiAxisController
                {
                    usage
                } else {
                    error!("Failed to get valid device: {:?}", usage);
                    return None;
                }
            }
            None => {
                error!("Failed to get usage of device");
                return None;
            }
        };

        let name = device.get_name().unwrap_or_else(|| {
            warn!("Failed to get name of device");
            "Unknown".into()
        });

        let uuid = match Self::create_uuid(&device) {
            Some(uuid) => uuid,
            None => Uuid::nil(),
        };

        let mut gamepad = Gamepad {
            name,
            uuid,
            entry_id,
            location_id,
            page,
            usage,
            axes_info: VecMap::with_capacity(8),
            axes: Vec::with_capacity(8),
            buttons: Vec::with_capacity(16),
            is_connected: true,
        };
        gamepad.collect_axes_and_buttons(&device.get_elements());

        Some(gamepad)
    }

    fn create_uuid(device: &IOHIDDevice) -> Option<Uuid> {
        // SDL always uses USB bus for UUID
        let bustype = u32::to_be(0x03);

        let vendor_id = match device.get_vendor_id() {
            Some(vendor_id) => vendor_id.to_be(),
            None => {
                warn!("Failed to get vendor id of device");
                0
            }
        };

        let product_id = match device.get_product_id() {
            Some(product_id) => product_id.to_be(),
            None => {
                warn!("Failed to get product id of device");
                0
            }
        };

        let version = match device.get_version() {
            Some(version) => version.to_be(),
            None => {
                warn!("Failed to get version of device");
                0
            }
        };

        if vendor_id == 0 && product_id == 0 && version == 0 {
            None
        } else {
            match Uuid::from_fields(
                bustype,
                vendor_id,
                0,
                &[
                    (product_id >> 8) as u8,
                    product_id as u8,
                    0,
                    0,
                    (version >> 8) as u8,
                    version as u8,
                    0,
                    0,
                ],
            ) {
                Ok(uuid) => Some(uuid),
                Err(error) => {
                    warn!("Failed to create uuid of device: {:?}", error.to_string());
                    None
                }
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn power_info(&self) -> PowerInfo {
        PowerInfo::Unknown
    }

    pub fn is_ff_supported(&self) -> bool {
        false
    }

    /// Creates Ffdevice corresponding to this gamepad.
    pub fn ff_device(&self) -> Option<FfDevice> {
        Some(FfDevice)
    }

    pub fn buttons(&self) -> &[EvCode] {
        &self.buttons
    }

    pub fn axes(&self) -> &[EvCode] {
        &self.axes
    }

    pub(crate) fn axis_info(&self, nec: EvCode) -> Option<&AxisInfo> {
        self.axes_info.get(nec.usage as usize)
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    fn collect_axes_and_buttons(&mut self, elements: &Vec<IOHIDElement>) {
        let mut cookies = Vec::new();

        self.collect_axes(&elements, &mut cookies);
        self.axes.sort_by_key(|axis| axis.usage);

        self.collect_buttons(&elements, &mut cookies);
        self.buttons.sort_by_key(|button| button.usage);
    }

    fn collect_axes(&mut self, elements: &Vec<IOHIDElement>, cookies: &mut Vec<u32>) {
        for element in elements {
            let type_ = element.get_type();
            let cookie = element.get_cookie();
            let page = element.get_page();
            let usage = element.get_usage();

            if IOHIDElement::is_collection_type(type_) {
                let children = element.get_children();
                self.collect_axes(&children, cookies);
            } else if IOHIDElement::is_axis(type_, page, usage) && !cookies.contains(&cookie) {
                cookies.push(cookie);
                self.axes_info.insert(
                    usage as usize,
                    AxisInfo {
                        min: element.get_logical_min() as _,
                        max: element.get_logical_max() as _,
                        deadzone: None,
                    },
                );
                self.axes.push(EvCode::new(page, usage));
            } else if IOHIDElement::is_hat(type_, page, usage) && !cookies.contains(&cookie) {
                cookies.push(cookie);
                self.axes_info.insert(
                    usage as usize,
                    AxisInfo {
                        min: -1,
                        max: 1,
                        deadzone: None,
                    },
                );
                // All hat switches are translated into *two* axes
                self.axes_info.insert(
                    (usage + 1) as usize, // "+ 1" is assumed for usage of 2nd hat switch axis
                    AxisInfo {
                        min: -1,
                        max: 1,
                        deadzone: None,
                    },
                );
            }
        }
    }

    fn collect_buttons(&mut self, elements: &Vec<IOHIDElement>, cookies: &mut Vec<u32>) {
        for element in elements {
            let type_ = element.get_type();
            let cookie = element.get_cookie();
            let page = element.get_page();
            let usage = element.get_usage();

            if IOHIDElement::is_collection_type(type_) {
                let children = element.get_children();
                self.collect_buttons(&children, cookies);
            } else if IOHIDElement::is_button(type_, page, usage) && !cookies.contains(&cookie) {
                cookies.push(cookie);
                self.buttons.push(EvCode::new(page, usage));
            }
        }
    }
}

#[derive(Debug)]
struct DeviceInfo {
    entry_id: u64,
    location_id: u32,
    is_connected: bool,
}
#[cfg(feature = "serde-serialize")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde-serialize", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EvCode {
    page: u32,
    usage: u32,
}

impl EvCode {
    fn new(page: u32, usage: u32) -> Self {
        EvCode { page, usage }
    }

    pub fn into_u32(self) -> u32 {
        self.page << 16 | self.usage
    }
}

impl From<IOHIDElement> for crate::EvCode {
    fn from(e: IOHIDElement) -> Self {
        crate::EvCode(EvCode {
            page: e.get_page(),
            usage: e.get_usage(),
        })
    }
}

impl Display for EvCode {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self.page {
            PAGE_GENERIC_DESKTOP => f.write_str("GENERIC_DESKTOP")?,
            PAGE_BUTTON => f.write_str("BUTTON")?,
            page => f.write_fmt(format_args!("PAGE_{}", page))?,
        }
        f.write_fmt(format_args!("({})", self.usage))
    }
}

pub mod native_ev_codes {
    use super::*;

    pub const AXIS_LSTICKX: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_LSTICKX,
    };
    pub const AXIS_LSTICKY: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_LSTICKY,
    };
    pub const AXIS_LEFTZ: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_LEFTZ,
    };
    pub const AXIS_RSTICKX: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_RSTICKX,
    };
    pub const AXIS_RSTICKY: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_RSTICKY,
    };
    pub const AXIS_RIGHTZ: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_RIGHTZ,
    };
    pub const AXIS_DPADX: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_DPADX,
    };
    pub const AXIS_DPADY: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_DPADY,
    };
    pub const AXIS_RT: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_RT,
    };
    pub const AXIS_LT: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_LT,
    };
    pub const AXIS_RT2: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_RT2,
    };
    pub const AXIS_LT2: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_LT2,
    };

    pub const BTN_SOUTH: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_SOUTH,
    };
    pub const BTN_EAST: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_EAST,
    };
    pub const BTN_C: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_C,
    };
    pub const BTN_NORTH: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_NORTH,
    };
    pub const BTN_WEST: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_WEST,
    };
    pub const BTN_Z: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_Z,
    };
    pub const BTN_LT: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_LT,
    };
    pub const BTN_RT: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_RT,
    };
    pub const BTN_LT2: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_LT2,
    };
    pub const BTN_RT2: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_RT2,
    };
    pub const BTN_SELECT: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_SELECT,
    };
    pub const BTN_START: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_START,
    };
    pub const BTN_MODE: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_MODE,
    };
    pub const BTN_LTHUMB: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_LTHUMB,
    };
    pub const BTN_RTHUMB: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_RTHUMB,
    };

    pub const BTN_DPAD_UP: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_DPAD_UP,
    };
    pub const BTN_DPAD_DOWN: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_DPAD_DOWN,
    };
    pub const BTN_DPAD_LEFT: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_DPAD_LEFT,
    };
    pub const BTN_DPAD_RIGHT: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_DPAD_RIGHT,
    };
}

extern "C" fn device_matching_cb(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    value: IOHIDDeviceRef,
) {
    let (tx, device_infos): &(
        Sender<(Event, Option<IOHIDDevice>)>,
        Arc<Mutex<Vec<DeviceInfo>>>,
    ) = unsafe { &*(context as *mut _) };
    let device = match IOHIDDevice::new(value) {
        Some(device) => device,
        None => {
            error!("Failed to get device");
            return;
        }
    };

    let io_service = match device.get_service() {
        Some(io_service) => io_service,
        None => {
            error!("Failed to get device service");
            return;
        }
    };

    let entry_id = match io_service.get_registry_entry_id() {
        Some(entry_id) => entry_id,
        None => {
            error!("Failed to get entry id of device");
            return;
        }
    };

    let mut device_infos = device_infos.lock().unwrap();
    let id = match device_infos
        .iter()
        .position(|info| info.entry_id == entry_id && info.is_connected)
    {
        Some(id) => {
            info!("Device is already registered: {:?}", entry_id);
            id
        }
        None => {
            let location_id = match device.get_location_id() {
                Some(location_id) => location_id,
                None => {
                    error!("Failed to get location id of device");
                    return;
                }
            };

            device_infos.push(DeviceInfo {
                entry_id: entry_id,
                location_id: location_id,
                is_connected: true,
            });

            device_infos.len() - 1
        }
    };
    let _ = tx.send((Event::new(id, EventType::Connected), Some(device)));
}

extern "C" fn device_removal_cb(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    value: IOHIDDeviceRef,
) {
    let (tx, device_infos): &(
        Sender<(Event, Option<IOHIDDevice>)>,
        Arc<Mutex<Vec<DeviceInfo>>>,
    ) = unsafe { &*(context as *mut _) };

    let device = match IOHIDDevice::new(value) {
        Some(device) => device,
        None => {
            error!("Failed to get device");
            return;
        }
    };

    let location_id = match device.get_location_id() {
        Some(location_id) => location_id,
        None => {
            error!("Failed to get location id of device");
            return;
        }
    };

    let device_infos = device_infos.lock().unwrap();
    let id = match device_infos
        .iter()
        .position(|info| info.location_id == location_id && info.is_connected)
    {
        Some(id) => id,
        None => {
            warn!("Failed to find device: {:?}", location_id);
            return;
        }
    };

    let _ = tx.send((Event::new(id, EventType::Disconnected), None));
}

extern "C" fn input_value_cb(
    context: *mut c_void,
    _result: IOReturn,
    sender: *mut c_void,
    value: IOHIDValueRef,
) {
    let (tx, device_infos): &(
        Sender<(Event, Option<IOHIDDevice>)>,
        Arc<Mutex<Vec<DeviceInfo>>>,
    ) = unsafe { &*(context as *mut _) };

    let device = match IOHIDDevice::new(sender as _) {
        Some(device) => device,
        None => {
            error!("Failed to get device");
            return;
        }
    };

    let io_service = match device.get_service() {
        Some(io_service) => io_service,
        None => {
            error!("Failed to get device service");
            return;
        }
    };

    let entry_id = match io_service.get_registry_entry_id() {
        Some(entry_id) => entry_id,
        None => {
            error!("Failed to get entry id of device");
            return;
        }
    };

    let device_infos = device_infos.lock().unwrap();
    let id = match device_infos
        .iter()
        .position(|info| info.entry_id == entry_id && info.is_connected)
    {
        Some(id) => id,
        None => {
            warn!("Failed to find device: {:?}", entry_id);
            return;
        }
    };

    let value = match IOHIDValue::new(value) {
        Some(value) => value,
        None => {
            error!("Failed to get value");
            return;
        }
    };

    let element = match value.get_element() {
        Some(element) => element,
        None => {
            error!("Failed to get element of value");
            return;
        }
    };

    let type_ = element.get_type();
    let page = element.get_page();
    let usage = element.get_usage();

    if IOHIDElement::is_axis(type_, page, usage) {
        let event = Event::new(
            id,
            EventType::AxisValueChanged(
                value.get_value() as i32,
                crate::EvCode(EvCode {
                    page: page,
                    usage: usage,
                }),
            ),
        );
        let _ = tx.send((event, None));
    } else if IOHIDElement::is_button(type_, page, usage) {
        if value.get_value() == 0 {
            let event = Event::new(
                id,
                EventType::ButtonReleased(crate::EvCode(EvCode {
                    page: page,
                    usage: usage,
                })),
            );
            let _ = tx.send((event, None));
        } else {
            let event = Event::new(
                id,
                EventType::ButtonPressed(crate::EvCode(EvCode {
                    page: page,
                    usage: usage,
                })),
            );
            let _ = tx.send((event, None));
        }
    } else if IOHIDElement::is_hat(type_, page, usage) {
        // Hat switch values are reported with a range of usually 8 numbers (sometimes 4). The logic
        // below uses the reported min/max values of that range to map that onto a range of 0-7 for
        // the directions (and any other value indicates the center position). Lucky for us, they
        // always start with "up" as the lowest number and proceed clockwise. See similar handling
        // here https://github.com/spurious/SDL-mirror/blob/094b2f68dd7fc9af167f905e10625e103a131459/src/joystick/darwin/SDL_sysjoystick.c#L976-L1028
        //
        //          up
        //       7  0  1
        //        \ | /
        // left 6 - ? - 2 right       (After mapping)
        //        / | \
        //       5  4  3
        //         down
        let range = element.get_logical_max() - element.get_logical_min() + 1;
        let shifted_value = value.get_value() - element.get_logical_min();
        let dpad_value = match range {
            4 => shifted_value * 2, // 4-position hat switch - scale it up to 8
            8 => shifted_value,     // 8-position hat switch - no adjustment necessary
            _ => -1, // Neither 4 nor 8 positions, we don't know what to do - default to centered
        };
        // At this point, the value should be normalized to the 0-7 directional values (or center
        // for any other value). The dpad is a hat switch on macOS, but on other platforms dpads are
        // either buttons or a pair of axes that get converted to button events by the
        // `axis_dpad_to_button` filter.  We will emulate axes here and let that filter do the
        // button conversion, because it is safer and easier than making separate logic for button
        // conversion that may diverge in subtle ways from the axis conversion logic.  The most
        // practical outcome of this conversion is that there are extra "released" axis events for
        // the unused axis. For example, pressing just "up" will also give you a "released" event
        // for either the left or right button, even if it wasn't pressed before pressing "up".
        let x_axis_value = match dpad_value {
            5 | 6 | 7 => -1, // left
            1 | 2 | 3 => 1,  // right
            _ => 0,
        };
        // Since we're emulating an inverted macOS gamepad axis, down is positive and up is negative
        let y_axis_value = match dpad_value {
            3 | 4 | 5 => 1,  // down
            0 | 1 | 7 => -1, // up
            _ => 0,
        };

        let x_axis_event = Event::new(
            id,
            EventType::AxisValueChanged(
                x_axis_value,
                crate::EvCode(EvCode {
                    page,
                    usage: USAGE_AXIS_DPADX,
                }),
            ),
        );
        let y_axis_event = Event::new(
            id,
            EventType::AxisValueChanged(
                y_axis_value,
                crate::EvCode(EvCode {
                    page,
                    usage: USAGE_AXIS_DPADY,
                }),
            ),
        );

        let _ = tx.send((x_axis_event, None));
        let _ = tx.send((y_axis_event, None));
    }
}
