// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use libc as c;
use libudev_sys as ud;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

#[derive(Debug)]
pub struct Udev(*mut ud::udev);

impl Udev {
    pub fn new() -> Option<Self> {
        let u = unsafe { ud::udev_new() };
        if u.is_null() {
            None
        } else {
            Some(Udev(u))
        }
    }

    pub fn enumerate(&self) -> Option<Enumerate> {
        let en = unsafe { ud::udev_enumerate_new(self.0) };
        if en.is_null() {
            None
        } else {
            let en = Enumerate(en);
            Some(en)
        }
    }
}

impl Drop for Udev {
    fn drop(&mut self) {
        unsafe {
            ud::udev_unref(self.0);
        }
    }
}

impl Clone for Udev {
    fn clone(&self) -> Self {
        Udev(unsafe { ud::udev_ref(self.0) })
    }
}

pub struct Enumerate(*mut ud::udev_enumerate);

impl Enumerate {
    pub fn scan_devices(&self) {
        // TODO: Check for error
        let _ = unsafe { ud::udev_enumerate_scan_devices(self.0) };
    }

    pub fn add_match_property(&self, key: &CStr, val: &CStr) {
        // TODO: Check for error
        unsafe {
            ud::udev_enumerate_add_match_property(self.0, key.as_ptr(), val.as_ptr());
        }
    }

    pub fn iter(&self) -> DeviceIterator {
        DeviceIterator(unsafe { ud::udev_enumerate_get_list_entry(self.0) })
    }
}

impl Drop for Enumerate {
    fn drop(&mut self) {
        unsafe {
            ud::udev_enumerate_unref(self.0);
        }
    }
}

pub struct DeviceIterator(*mut ud::udev_list_entry);

impl Iterator for DeviceIterator {
    type Item = CString;

    fn next(&mut self) -> Option<CString> {
        if self.0.is_null() {
            None
        } else {
            let p_name = unsafe { ud::udev_list_entry_get_name(self.0) };
            let name = if p_name.is_null() {
                return None;
            } else {
                unsafe { CStr::from_ptr(p_name).to_owned() }
            };
            self.0 = unsafe { ud::udev_list_entry_get_next(self.0) };
            Some(name)
        }
    }
}

pub struct Device(*mut ud::udev_device);

impl Device {
    pub fn from_syspath(udev: &Udev, path: &CStr) -> Option<Self> {
        let dev = unsafe { ud::udev_device_new_from_syspath(udev.0, path.as_ptr()) };
        if dev.is_null() {
            None
        } else {
            Some(Device(dev))
        }
    }

    pub fn syspath(&self) -> &CStr {
        // Always returns cstring
        unsafe { CStr::from_ptr(ud::udev_device_get_syspath(self.0)) }
    }

    pub fn devnode(&self) -> Option<&CStr> {
        unsafe {
            let s = ud::udev_device_get_devnode(self.0);
            if s.is_null() {
                None
            } else {
                Some(CStr::from_ptr(s))
            }
        }
    }

    #[allow(dead_code)]
    pub fn properties(&self) -> PropertyIterator {
        let prop = unsafe { ud::udev_device_get_properties_list_entry(self.0) };
        PropertyIterator(prop)
    }

    pub fn action(&self) -> Option<&CStr> {
        unsafe {
            let s = ud::udev_device_get_action(self.0);
            if s.is_null() {
                None
            } else {
                Some(CStr::from_ptr(s))
            }
        }
    }

    pub fn property_value(&self, key: &CStr) -> Option<&CStr> {
        unsafe {
            let s = ud::udev_device_get_property_value(self.0, key.as_ptr());
            if s.is_null() {
                None
            } else {
                Some(CStr::from_ptr(s))
            }
        }
    }
}

impl Clone for Device {
    fn clone(&self) -> Self {
        unsafe { Device(ud::udev_device_ref(self.0)) }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            ud::udev_device_unref(self.0);
        }
    }
}

#[allow(dead_code)]
pub struct PropertyIterator(*mut ud::udev_list_entry);

impl Iterator for PropertyIterator {
    type Item = (String, String);

    fn next(&mut self) -> Option<(String, String)> {
        if self.0.is_null() {
            None
        } else {
            let p_name = unsafe { ud::udev_list_entry_get_name(self.0) };
            let p_val = unsafe { ud::udev_list_entry_get_value(self.0) };

            let name = if p_name.is_null() {
                return None;
            } else {
                unsafe { CStr::from_ptr(p_name).to_string_lossy().into_owned() }
            };

            let value = if p_val.is_null() {
                return None;
            } else {
                unsafe { CStr::from_ptr(p_val).to_string_lossy().into_owned() }
            };

            self.0 = unsafe { ud::udev_list_entry_get_next(self.0) };
            Some((name, value))
        }
    }
}

#[derive(Debug)]
pub struct Monitor(*mut ud::udev_monitor);

impl Monitor {
    pub fn new(udev: &Udev) -> Option<Self> {
        unsafe {
            let monitor =
                ud::udev_monitor_new_from_netlink(udev.0, b"udev\0".as_ptr() as *const c_char);
            if monitor.is_null() {
                None
            } else {
                ud::udev_monitor_filter_add_match_subsystem_devtype(
                    monitor,
                    b"input\0".as_ptr() as *const c_char,
                    ptr::null(),
                );
                ud::udev_monitor_enable_receiving(monitor);
                Some(Monitor(monitor))
            }
        }
    }

    pub fn hotplug_available(&self) -> bool {
        unsafe {
            let mut fds = c::pollfd {
                fd: ud::udev_monitor_get_fd(self.0),
                events: c::POLLIN,
                revents: 0,
            };
            (c::poll(&mut fds, 1, 0) == 1) && (fds.revents & c::POLLIN != 0)
        }
    }

    pub fn device(&self) -> Device {
        Device(unsafe { ud::udev_monitor_receive_device(self.0) })
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        unsafe {
            ud::udev_monitor_unref(self.0);
        }
    }
}
