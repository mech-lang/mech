// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::ff::Device as FfDevice;
use super::ioctl;
use super::ioctl::{input_absinfo, input_event};
use super::udev::*;
use crate::utils;
use crate::{AxisInfo, Event, EventType};
use crate::{PlatformError, PowerInfo};

use libc as c;
use uuid::Uuid;
use vec_map::VecMap;

use std::error;
use std::ffi::CStr;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::mem::{self, MaybeUninit};
use std::ops::Index;
use std::os::raw::c_char;
use std::str;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Gilrs {
    gamepads: Vec<Gamepad>,
    monitor: Monitor,
    event_counter: usize,
}

impl Gilrs {
    pub(crate) fn new() -> Result<Self, PlatformError> {
        let mut gamepads = Vec::new();

        let udev = match Udev::new() {
            Some(udev) => udev,
            None => {
                return Err(PlatformError::Other(Box::new(Error::UdevCtx)));
            }
        };
        let en = match udev.enumerate() {
            Some(en) => en,
            None => {
                return Err(PlatformError::Other(Box::new(Error::UdevEnumerate)));
            }
        };

        unsafe { en.add_match_property(cstr_new(b"ID_INPUT_JOYSTICK\0"), cstr_new(b"1\0")) }
        en.scan_devices();

        for dev in en.iter() {
            if let Some(dev) = Device::from_syspath(&udev, &dev) {
                if let Some(gamepad) = Gamepad::open(&dev) {
                    gamepads.push(gamepad);
                }
            }
        }

        let monitor = match Monitor::new(&udev) {
            Some(m) => m,
            None => return Err(PlatformError::Other(Box::new(Error::UdevMonitor))),
        };

        Ok(Gilrs {
            gamepads,
            monitor,
            event_counter: 0,
        })
    }

    pub(crate) fn next_event(&mut self) -> Option<Event> {
        if let Some(event) = self.handle_hotplug() {
            return Some(event);
        }

        loop {
            let gamepad = match self.gamepads.get_mut(self.event_counter) {
                Some(gp) => gp,
                None => {
                    self.event_counter = 0;
                    return None;
                }
            };

            if !gamepad.is_connected {
                self.event_counter += 1;
                continue;
            }

            match gamepad.event() {
                Some((event, time)) => {
                    return Some(Event {
                        id: self.event_counter,
                        event,
                        time,
                    });
                }
                None => {
                    self.event_counter += 1;
                    continue;
                }
            };
        }
    }

    pub fn gamepad(&self, id: usize) -> Option<&Gamepad> {
        self.gamepads.get(id)
    }

    pub fn last_gamepad_hint(&self) -> usize {
        self.gamepads.len()
    }

    fn handle_hotplug(&mut self) -> Option<Event> {
        while self.monitor.hotplug_available() {
            let dev = self.monitor.device();

            unsafe {
                if let Some(val) = dev.property_value(cstr_new(b"ID_INPUT_JOYSTICK\0")) {
                    if val != cstr_new(b"1\0") {
                        continue;
                    }
                } else {
                    continue;
                }

                let action = match dev.action() {
                    Some(a) => a,
                    None => continue,
                };

                if action == cstr_new(b"add\0") {
                    if let Some(gamepad) = Gamepad::open(&dev) {
                        if let Some(id) = self
                            .gamepads
                            .iter()
                            .position(|gp| gp.uuid() == gamepad.uuid && !gp.is_connected)
                        {
                            self.gamepads[id] = gamepad;
                            return Some(Event::new(id, EventType::Connected));
                        } else {
                            self.gamepads.push(gamepad);
                            return Some(Event::new(self.gamepads.len() - 1, EventType::Connected));
                        }
                    }
                } else if action == cstr_new(b"remove\0") {
                    if let Some(devnode) = dev.devnode() {
                        if let Some(id) = self
                            .gamepads
                            .iter()
                            .position(|gp| is_eq_cstr_str(devnode, &gp.devpath) && gp.is_connected)
                        {
                            self.gamepads[id].disconnect();
                            return Some(Event::new(id, EventType::Disconnected));
                        } else {
                            debug!("Could not find disconnected gamepad {:?}", devnode);
                        }
                    }
                }
            }
        }
        None
    }
}

fn is_eq_cstr_str(l: &CStr, r: &str) -> bool {
    unsafe {
        let mut l_ptr = l.as_ptr();
        let mut r_ptr = r.as_ptr();
        let end = r_ptr.add(r.len());
        while *l_ptr != 0 && r_ptr != end {
            if *l_ptr != *r_ptr as c_char {
                return false;
            }
            l_ptr = l_ptr.offset(1);
            r_ptr = r_ptr.offset(1);
        }

        *l_ptr == 0 && r_ptr == end
    }
}

#[derive(Debug, Clone)]
struct AxesInfo {
    info: VecMap<AxisInfo>,
}

impl AxesInfo {
    fn new(fd: i32) -> Self {
        let mut map = VecMap::new();

        unsafe {
            let mut abs_bits = [0u8; (ABS_MAX / 8) as usize + 1];
            ioctl::eviocgbit(
                fd,
                u32::from(EV_ABS),
                abs_bits.len() as i32,
                abs_bits.as_mut_ptr(),
            );

            for axis in Gamepad::find_axes(&abs_bits) {
                let mut info = input_absinfo::default();
                ioctl::eviocgabs(fd, u32::from(axis.code), &mut info);
                map.insert(
                    axis.code as usize,
                    AxisInfo {
                        min: info.minimum,
                        max: info.maximum,
                        deadzone: Some(info.flat as u32),
                    },
                );
            }
        }

        AxesInfo { info: map }
    }
}

impl Index<u16> for AxesInfo {
    type Output = AxisInfo;

    fn index(&self, i: u16) -> &Self::Output {
        &self.info[i as usize]
    }
}

#[derive(Debug)]
pub struct Gamepad {
    fd: i32,
    axes_info: AxesInfo,
    ff_supported: bool,
    devpath: String,
    name: String,
    uuid: Uuid,
    // TODO: path or RefCell<File>
    bt_capacity_fd: i32,
    // TODO: path or RefCell<File>
    bt_status_fd: i32,
    axes_values: VecMap<i32>,
    buttons_values: VecMap<bool>,
    events: Vec<input_event>,
    axes: Vec<EvCode>,
    buttons: Vec<EvCode>,
    is_connected: bool,
}

impl Gamepad {
    fn open(dev: &Device) -> Option<Gamepad> {
        let path = match dev.devnode() {
            Some(path) => path,
            None => return None,
        };

        if unsafe { !c::strstr(path.as_ptr(), b"js\0".as_ptr() as *const c_char).is_null() } {
            trace!("Device {:?} is js interface, ignoring.", path);
            return None;
        }

        let fd = unsafe { c::open(path.as_ptr(), c::O_RDWR | c::O_NONBLOCK) };
        if fd < 0 {
            error!("Failed to open {:?}", path);
            return None;
        }

        let uuid = match Self::create_uuid(fd) {
            Some(uuid) => uuid,
            None => {
                error!("Failed to get id of device {:?}", path);
                unsafe {
                    c::close(fd);
                }
                return None;
            }
        };

        let name = Self::get_name(fd).unwrap_or_else(|| {
            error!("Failed to get name od device {:?}", path);
            "Unknown".into()
        });

        let axesi = AxesInfo::new(fd);
        let ff_supported = Self::test_ff(fd);
        let (cap, status) = Self::battery_fd(&dev);

        let mut gamepad = Gamepad {
            fd,
            axes_info: axesi,
            ff_supported,
            devpath: path.to_string_lossy().into_owned(),
            name,
            uuid,
            bt_capacity_fd: cap,
            bt_status_fd: status,
            axes_values: VecMap::new(),
            buttons_values: VecMap::new(),
            events: Vec::new(),
            axes: Vec::new(),
            buttons: Vec::new(),
            is_connected: true,
        };

        gamepad.collect_axes_and_buttons();

        if !gamepad.is_gamepad() {
            warn!(
                "{:?} doesn't have at least 1 button and 2 axes, ignoring.",
                path
            );
            return None;
        }

        info!("Gamepad {} ({}) connected.", gamepad.devpath, gamepad.name);
        debug!(
            "Gamepad {}: uuid: {}, ff_supported: {}, axes: {:?}, buttons: {:?}, axes_info: {:?}",
            gamepad.devpath,
            gamepad.uuid,
            gamepad.ff_supported,
            gamepad.axes,
            gamepad.buttons,
            gamepad.axes_info
        );

        Some(gamepad)
    }

    fn collect_axes_and_buttons(&mut self) {
        let mut key_bits = [0u8; (KEY_MAX / 8) as usize + 1];
        let mut abs_bits = [0u8; (ABS_MAX / 8) as usize + 1];

        unsafe {
            ioctl::eviocgbit(
                self.fd,
                u32::from(EV_KEY),
                key_bits.len() as i32,
                key_bits.as_mut_ptr(),
            );
            ioctl::eviocgbit(
                self.fd,
                u32::from(EV_ABS),
                abs_bits.len() as i32,
                abs_bits.as_mut_ptr(),
            );
        }

        self.buttons = Self::find_buttons(&key_bits, false);
        self.axes = Self::find_axes(&abs_bits);
    }

    fn get_name(fd: i32) -> Option<String> {
        unsafe {
            let mut namebuff: [MaybeUninit<u8>; 128] = MaybeUninit::uninit().assume_init();
            if ioctl::eviocgname(fd, &mut namebuff).is_err() {
                None
            } else {
                Some(
                    CStr::from_ptr(namebuff.as_ptr() as *const c_char)
                        .to_string_lossy()
                        .into_owned(),
                )
            }
        }
    }

    fn test_ff(fd: i32) -> bool {
        unsafe {
            let mut ff_bits = [0u8; (FF_MAX / 8) as usize + 1];
            if ioctl::eviocgbit(
                fd,
                u32::from(EV_FF),
                ff_bits.len() as i32,
                ff_bits.as_mut_ptr(),
            ) >= 0
            {
                utils::test_bit(FF_SQUARE, &ff_bits)
                    && utils::test_bit(FF_TRIANGLE, &ff_bits)
                    && utils::test_bit(FF_SINE, &ff_bits)
                    && utils::test_bit(FF_GAIN, &ff_bits)
            } else {
                false
            }
        }
    }

    fn is_gamepad(&self) -> bool {
        // TODO: improve it (for example check for buttons in range)
        !self.buttons.is_empty() && self.axes.len() >= 2
    }

    fn create_uuid(fd: i32) -> Option<Uuid> {
        let iid = unsafe {
            let mut iid = MaybeUninit::<ioctl::input_id>::uninit();
            if ioctl::eviocgid(fd, iid.as_mut_ptr()).is_err() {
                return None;
            }

            iid.assume_init()
        };
        Some(create_uuid(iid))
    }

    fn find_buttons(key_bits: &[u8], only_gamepad_btns: bool) -> Vec<EvCode> {
        let mut buttons = Vec::with_capacity(16);

        for bit in BTN_MISC..BTN_MOUSE {
            if utils::test_bit(bit, &key_bits) {
                buttons.push(EvCode::new(EV_KEY, bit));
            }
        }
        for bit in BTN_JOYSTICK..(key_bits.len() as u16 * 8) {
            if utils::test_bit(bit, &key_bits) {
                buttons.push(EvCode::new(EV_KEY, bit));
            }
        }

        if !only_gamepad_btns {
            for bit in 0..BTN_MISC {
                if utils::test_bit(bit, &key_bits) {
                    buttons.push(EvCode::new(EV_KEY, bit));
                }
            }
            for bit in BTN_MOUSE..BTN_JOYSTICK {
                if utils::test_bit(bit, &key_bits) {
                    buttons.push(EvCode::new(EV_KEY, bit));
                }
            }
        }

        buttons
    }

    fn find_axes(abs_bits: &[u8]) -> Vec<EvCode> {
        let mut axes = Vec::with_capacity(8);

        for bit in 0..(abs_bits.len() * 8) {
            if utils::test_bit(bit as u16, &abs_bits) {
                axes.push(EvCode::new(EV_ABS, bit as u16));
            }
        }

        axes
    }

    fn battery_fd(dev: &Device) -> (i32, i32) {
        use std::ffi::OsStr;
        use std::fs::{self, File};
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::io::IntoRawFd;
        use std::path::Path;

        let syspath = Path::new(OsStr::from_bytes(dev.syspath().to_bytes()));
        // Returned syspath points to <device path>/input/inputXX/eventXX. First "device" is
        // symlink to inputXX, second to actual device root.
        let syspath = syspath.join("device/device/power_supply");
        if let Ok(mut read_dir) = fs::read_dir(syspath) {
            if let Some(Ok(bat_entry)) = read_dir.next() {
                if let Ok(cap) = File::open(bat_entry.path().join("capacity")) {
                    if let Ok(status) = File::open(bat_entry.path().join("status")) {
                        return (cap.into_raw_fd(), status.into_raw_fd());
                    }
                }
            }
        }
        (-1, -1)
    }

    fn event(&mut self) -> Option<(EventType, SystemTime)> {
        let mut skip = false;
        // Skip all unknown events and return Option on first know event or when there is no more
        // events to read. Returning None on unknown event breaks iterators.
        loop {
            let event = match self.next_event() {
                Some(e) => e,
                None => return None,
            };

            if skip {
                if event.type_ == EV_SYN && event.code == SYN_REPORT {
                    skip = false;
                    self.compare_state();
                }
                continue;
            }

            let ev = match event.type_ {
                EV_SYN if event.code == SYN_DROPPED => {
                    skip = true;
                    None
                }
                EV_KEY => {
                    self.buttons_values
                        .insert(event.code as usize, event.value == 1);
                    match event.value {
                        0 => Some(EventType::ButtonReleased(event.into())),
                        1 => Some(EventType::ButtonPressed(event.into())),
                        _ => None,
                    }
                }
                EV_ABS => {
                    self.axes_values.insert(event.code as usize, event.value);
                    Some(EventType::AxisValueChanged(event.value, event.into()))
                }
                _ => {
                    trace!("Skipping event {:?}", event);
                    None
                }
            };

            if let Some(ev) = ev {
                let dur = Duration::new(event.time.tv_sec as u64, event.time.tv_usec as u32 * 1000);

                return Some((ev, UNIX_EPOCH + dur));
            }
        }
    }

    fn next_event(&mut self) -> Option<input_event> {
        if !self.events.is_empty() {
            self.events.pop()
        } else {
            unsafe {
                let mut event_buf: [MaybeUninit<ioctl::input_event>; 12] =
                    MaybeUninit::uninit().assume_init();
                let size = mem::size_of::<ioctl::input_event>();
                let n = c::read(
                    self.fd,
                    event_buf.as_mut_ptr() as *mut c::c_void,
                    size * event_buf.len(),
                );

                if n == -1 || n == 0 {
                    // Nothing to read (non-blocking IO)
                    None
                } else if n % size as isize != 0 {
                    error!("Unexpected read of size {}", n);
                    None
                } else {
                    let n = n as usize / size;
                    trace!("Got {} new events", n);
                    for ev in event_buf[1..n].iter().rev() {
                        self.events.push(ev.assume_init());
                    }

                    Some(event_buf[0].assume_init())
                }
            }
        }
    }

    fn compare_state(&mut self) {
        let mut absinfo = input_absinfo::default();
        for axis in self.axes.iter().cloned() {
            let value = unsafe {
                ioctl::eviocgabs(self.fd, u32::from(axis.code), &mut absinfo);
                absinfo.value
            };

            if self
                .axes_values
                .get(axis.code as usize)
                .cloned()
                .unwrap_or(0)
                != value
            {
                self.events.push(input_event {
                    type_: EV_ABS,
                    code: axis.code,
                    value,
                    ..Default::default()
                });
            }
        }

        let mut buf = [0u8; KEY_MAX as usize / 8 + 1];
        unsafe {
            let _ = ioctl::eviocgkey(self.fd, &mut buf);
        }

        for btn in self.buttons.iter().cloned() {
            let val = utils::test_bit(btn.code, &buf);
            if self
                .buttons_values
                .get(btn.code as usize)
                .cloned()
                .unwrap_or(false)
                != val
            {
                self.events.push(input_event {
                    type_: EV_KEY,
                    code: btn.code,
                    value: val as i32,
                    ..Default::default()
                });
            }
        }
    }

    fn disconnect(&mut self) {
        unsafe {
            if self.fd >= 0 {
                c::close(self.fd);
            }
        }
        self.fd = -2;
        self.devpath.clear();
        self.is_connected = false;
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    pub fn power_info(&self) -> PowerInfo {
        if self.bt_capacity_fd > -1 && self.bt_status_fd > -1 {
            unsafe {
                let mut buff = [0u8; 15];
                c::lseek(self.bt_capacity_fd, 0, c::SEEK_SET);
                c::lseek(self.bt_status_fd, 0, c::SEEK_SET);

                let len = c::read(
                    self.bt_capacity_fd,
                    buff.as_mut_ptr() as *mut c::c_void,
                    buff.len(),
                ) as usize;

                if len > 0 {
                    let cap = match str::from_utf8_unchecked(&buff[..(len - 1)]).parse() {
                        Ok(cap) => cap,
                        Err(_) => {
                            error!(
                                "Failed to parse battery capacity: {}",
                                str::from_utf8_unchecked(&buff[..(len - 1)])
                            );
                            return PowerInfo::Unknown;
                        }
                    };

                    let len = c::read(
                        self.bt_status_fd,
                        buff.as_mut_ptr() as *mut c::c_void,
                        buff.len(),
                    ) as usize;

                    if len > 0 {
                        return match str::from_utf8_unchecked(&buff[..(len - 1)]) {
                            "Charging" => PowerInfo::Charging(cap),
                            "Discharging" => PowerInfo::Discharging(cap),
                            "Full" | "Not charging" => PowerInfo::Charged,
                            s => {
                                error!("Unknown battery status value: {}", s);
                                PowerInfo::Unknown
                            }
                        };
                    }
                }
            }
            PowerInfo::Unknown
        } else if self.fd > -1 {
            PowerInfo::Wired
        } else {
            PowerInfo::Unknown
        }
    }

    pub fn is_ff_supported(&self) -> bool {
        self.ff_supported
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn ff_device(&self) -> Option<FfDevice> {
        if self.is_ff_supported() {
            FfDevice::new(&self.devpath).ok()
        } else {
            None
        }
    }

    pub fn buttons(&self) -> &[EvCode] {
        &self.buttons
    }

    pub fn axes(&self) -> &[EvCode] {
        &self.axes
    }

    pub(crate) fn axis_info(&self, nec: EvCode) -> Option<&AxisInfo> {
        if nec.kind != EV_ABS {
            None
        } else {
            self.axes_info.info.get(nec.code as usize)
        }
    }
}

impl Drop for Gamepad {
    fn drop(&mut self) {
        unsafe {
            if self.fd >= 0 {
                c::close(self.fd);
            }
            if self.bt_capacity_fd >= 0 {
                c::close(self.bt_capacity_fd);
            }
            if self.bt_status_fd >= 0 {
                c::close(self.bt_status_fd);
            }
        }
    }
}

impl PartialEq for Gamepad {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

fn create_uuid(iid: ioctl::input_id) -> Uuid {
    let bus = (u32::from(iid.bustype)).to_be();
    let vendor = iid.vendor.to_be();
    let product = iid.product.to_be();
    let version = iid.version.to_be();
    Uuid::from_fields(
        bus,
        vendor,
        0,
        &[
            (product >> 8) as u8,
            product as u8,
            0,
            0,
            (version >> 8) as u8,
            version as u8,
            0,
            0,
        ],
    )
    .unwrap()
}

unsafe fn cstr_new(bytes: &[u8]) -> &CStr {
    CStr::from_bytes_with_nul_unchecked(bytes)
}

#[cfg(feature = "serde-serialize")]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde-serialize", derive(Serialize, Deserialize))]
pub struct EvCode {
    kind: u16,
    code: u16,
}

impl EvCode {
    fn new(kind: u16, code: u16) -> Self {
        EvCode { kind, code }
    }

    pub fn into_u32(self) -> u32 {
        u32::from(self.kind) << 16 | u32::from(self.code)
    }
}

impl From<input_event> for crate::EvCode {
    fn from(f: input_event) -> Self {
        crate::EvCode(EvCode {
            kind: f.type_,
            code: f.code,
        })
    }
}

impl Display for EvCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.kind {
            EV_SYN => f.write_str("SYN")?,
            EV_KEY => f.write_str("KEY")?,
            0x02 => f.write_str("REL")?,
            EV_ABS => f.write_str("ABS")?,
            0x04 => f.write_str("MSC")?,
            0x05 => f.write_str("SW")?,
            kind => f.write_fmt(format_args!("EV_TYPE_{}", kind))?,
        }

        f.write_fmt(format_args!("({})", self.code))
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(clippy::enum_variant_names)]
enum Error {
    UdevCtx,
    UdevEnumerate,
    UdevMonitor,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match *self {
            Error::UdevCtx => f.write_str("Failed to create udev context"),
            Error::UdevEnumerate => f.write_str("Failed to create udev enumerate object"),
            Error::UdevMonitor => f.write_str("Failed to create udev monitor."),
        }
    }
}

impl error::Error for Error {}

const KEY_MAX: u16 = 0x2ff;
#[allow(dead_code)]
const EV_MAX: u16 = 0x1f;
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const ABS_MAX: u16 = 0x3f;
const EV_FF: u16 = 0x15;

const SYN_REPORT: u16 = 0x00;
const SYN_DROPPED: u16 = 0x03;

const BTN_MISC: u16 = 0x100;
const BTN_MOUSE: u16 = 0x110;
const BTN_JOYSTICK: u16 = 0x120;
const BTN_SOUTH: u16 = 0x130;
const BTN_EAST: u16 = 0x131;
#[allow(dead_code)]
const BTN_C: u16 = 0x132;
const BTN_NORTH: u16 = 0x133;
const BTN_WEST: u16 = 0x134;
#[allow(dead_code)]
const BTN_Z: u16 = 0x135;
const BTN_TL: u16 = 0x136;
const BTN_TR: u16 = 0x137;
const BTN_TL2: u16 = 0x138;
const BTN_TR2: u16 = 0x139;
const BTN_SELECT: u16 = 0x13a;
const BTN_START: u16 = 0x13b;
const BTN_MODE: u16 = 0x13c;
const BTN_THUMBL: u16 = 0x13d;
const BTN_THUMBR: u16 = 0x13e;

const BTN_DPAD_UP: u16 = 0x220;
const BTN_DPAD_DOWN: u16 = 0x221;
const BTN_DPAD_LEFT: u16 = 0x222;
const BTN_DPAD_RIGHT: u16 = 0x223;

const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_Z: u16 = 0x02;
const ABS_RX: u16 = 0x03;
const ABS_RY: u16 = 0x04;
const ABS_RZ: u16 = 0x05;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;
const ABS_HAT1X: u16 = 0x12;
const ABS_HAT1Y: u16 = 0x13;
const ABS_HAT2X: u16 = 0x14;
const ABS_HAT2Y: u16 = 0x15;

const FF_MAX: u16 = FF_GAIN;
const FF_SQUARE: u16 = 0x58;
const FF_TRIANGLE: u16 = 0x59;
const FF_SINE: u16 = 0x5a;
const FF_GAIN: u16 = 0x60;

pub mod native_ev_codes {
    use super::*;

    pub const BTN_SOUTH: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_SOUTH,
    };
    pub const BTN_EAST: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_EAST,
    };
    pub const BTN_C: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_C,
    };
    pub const BTN_NORTH: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_NORTH,
    };
    pub const BTN_WEST: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_WEST,
    };
    pub const BTN_Z: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_Z,
    };
    pub const BTN_LT: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_TL,
    };
    pub const BTN_RT: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_TR,
    };
    pub const BTN_LT2: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_TL2,
    };
    pub const BTN_RT2: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_TR2,
    };
    pub const BTN_SELECT: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_SELECT,
    };
    pub const BTN_START: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_START,
    };
    pub const BTN_MODE: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_MODE,
    };
    pub const BTN_LTHUMB: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_THUMBL,
    };
    pub const BTN_RTHUMB: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_THUMBR,
    };
    pub const BTN_DPAD_UP: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_DPAD_UP,
    };
    pub const BTN_DPAD_DOWN: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_DPAD_DOWN,
    };
    pub const BTN_DPAD_LEFT: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_DPAD_LEFT,
    };
    pub const BTN_DPAD_RIGHT: EvCode = EvCode {
        kind: EV_KEY,
        code: super::BTN_DPAD_RIGHT,
    };

    pub const AXIS_LSTICKX: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_X,
    };
    pub const AXIS_LSTICKY: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_Y,
    };
    pub const AXIS_LEFTZ: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_Z,
    };
    pub const AXIS_RSTICKX: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_RX,
    };
    pub const AXIS_RSTICKY: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_RY,
    };
    pub const AXIS_RIGHTZ: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_RZ,
    };
    pub const AXIS_DPADX: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_HAT0X,
    };
    pub const AXIS_DPADY: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_HAT0Y,
    };
    pub const AXIS_RT: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_HAT1X,
    };
    pub const AXIS_LT: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_HAT1Y,
    };
    pub const AXIS_RT2: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_HAT2X,
    };
    pub const AXIS_LT2: EvCode = EvCode {
        kind: EV_ABS,
        code: super::ABS_HAT2Y,
    };
}

#[cfg(test)]
mod tests {
    use super::super::ioctl;
    use super::create_uuid;
    use uuid::Uuid;

    #[test]
    fn sdl_uuid() {
        let x = Uuid::parse_str("030000005e0400008e02000020200000").unwrap();
        let y = create_uuid(ioctl::input_id {
            bustype: 0x3,
            vendor: 0x045e,
            product: 0x028e,
            version: 0x2020,
        });
        assert_eq!(x, y);
    }
}
