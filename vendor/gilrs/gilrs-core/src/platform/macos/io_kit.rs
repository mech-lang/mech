// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use core_foundation::array::{
    kCFTypeArrayCallBacks, CFArray, CFArrayCallBacks, CFArrayGetCount, CFArrayGetValueAtIndex,
    __CFArray,
};
use core_foundation::base::{
    kCFAllocatorDefault, CFAllocatorRef, CFIndex, CFRelease, CFType, TCFType,
};
use core_foundation::dictionary::CFDictionary;
use core_foundation::impl_TCFType;
use core_foundation::number::CFNumber;
use core_foundation::runloop::{CFRunLoop, CFRunLoopMode};
use core_foundation::set::CFSetApplyFunction;
use core_foundation::string::{kCFStringEncodingUTF8, CFString, CFStringCreateWithCString};

use io_kit_sys::hid::base::{
    IOHIDDeviceCallback, IOHIDDeviceRef, IOHIDElementRef, IOHIDValueCallback, IOHIDValueRef,
};
use io_kit_sys::hid::device::*;
use io_kit_sys::hid::element::*;
use io_kit_sys::hid::keys::*;
use io_kit_sys::hid::manager::*;
use io_kit_sys::hid::usage_tables::*;
use io_kit_sys::hid::value::{
    IOHIDValueGetElement, IOHIDValueGetIntegerValue, IOHIDValueGetTypeID,
};
use io_kit_sys::ret::{kIOReturnSuccess, IOReturn};
use io_kit_sys::types::{io_service_t, IO_OBJECT_NULL};
use io_kit_sys::{IOObjectRelease, IOObjectRetain, IORegistryEntryGetRegistryEntryID};

use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;

#[repr(C)]
#[derive(Debug)]
pub struct IOHIDManager(IOHIDManagerRef);

pub type CFMutableArrayRef = *mut __CFArray;

extern "C" {
    pub fn CFArrayCreateMutable(
        allocator: CFAllocatorRef,
        capacity: CFIndex,
        callBacks: *const CFArrayCallBacks,
    ) -> CFMutableArrayRef;
    pub fn CFArrayAppendValue(theArray: CFMutableArrayRef, value: *const c_void);
}

impl_TCFType!(IOHIDManager, IOHIDManagerRef, IOHIDManagerGetTypeID);

impl IOHIDManager {
    pub fn new() -> Option<Self> {
        let manager = unsafe { IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDOptionsTypeNone) };

        if manager.is_null() {
            return None;
        }

        let matchers = CFArray::from_CFTypes(&[
            create_hid_device_matcher(kHIDPage_GenericDesktop, kHIDUsage_GD_Joystick),
            create_hid_device_matcher(kHIDPage_GenericDesktop, kHIDUsage_GD_GamePad),
            create_hid_device_matcher(kHIDPage_GenericDesktop, kHIDUsage_GD_MultiAxisController),
        ]);
        unsafe {
            IOHIDManagerSetDeviceMatchingMultiple(manager, matchers.as_concrete_TypeRef());
        };

        let ret = unsafe { IOHIDManagerOpen(manager, kIOHIDOptionsTypeNone) };

        if ret == kIOReturnSuccess {
            Some(IOHIDManager(manager))
        } else {
            unsafe { CFRelease(manager as _) };
            None
        }
    }

    pub fn open(&mut self) -> IOReturn {
        unsafe { IOHIDManagerOpen(self.0, kIOHIDOptionsTypeNone) }
    }

    pub fn close(&mut self) -> IOReturn {
        unsafe { IOHIDManagerClose(self.0, kIOHIDOptionsTypeNone) }
    }

    pub fn schedule_with_run_loop(&mut self, run_loop: CFRunLoop, run_loop_mode: CFRunLoopMode) {
        unsafe {
            IOHIDManagerScheduleWithRunLoop(self.0, run_loop.as_concrete_TypeRef(), run_loop_mode)
        }
    }

    pub fn unschedule_from_run_loop(&mut self, run_loop: CFRunLoop, run_loop_mode: CFRunLoopMode) {
        unsafe {
            IOHIDManagerUnscheduleFromRunLoop(self.0, run_loop.as_concrete_TypeRef(), run_loop_mode)
        }
    }

    pub fn register_device_matching_callback(
        &mut self,
        callback: IOHIDDeviceCallback,
        context: *mut c_void,
    ) {
        unsafe { IOHIDManagerRegisterDeviceMatchingCallback(self.0, callback, context) }
    }

    pub fn register_device_removal_callback(
        &mut self,
        callback: IOHIDDeviceCallback,
        context: *mut c_void,
    ) {
        unsafe { IOHIDManagerRegisterDeviceRemovalCallback(self.0, callback, context) }
    }

    pub fn register_input_value_callback(
        &mut self,
        callback: IOHIDValueCallback,
        context: *mut c_void,
    ) {
        unsafe { IOHIDManagerRegisterInputValueCallback(self.0, callback, context) }
    }

    pub fn get_devices(&mut self) -> Vec<IOHIDDevice> {
        let copied = unsafe { IOHIDManagerCopyDevices(self.0) };

        if copied.is_null() {
            return vec![];
        }

        let devices =
            unsafe { CFArrayCreateMutable(kCFAllocatorDefault, 0, &kCFTypeArrayCallBacks) };

        if devices.is_null() {
            unsafe { CFRelease(copied as _) };
            return vec![];
        }

        unsafe { CFSetApplyFunction(copied, cf_set_applier, devices as _) };
        unsafe { CFRelease(copied as _) };

        let device_count = unsafe { CFArrayGetCount(devices) };
        let mut vec = Vec::with_capacity(device_count as _);

        for i in 0..device_count {
            let device = unsafe { CFArrayGetValueAtIndex(devices, i) };

            if device.is_null() {
                continue;
            }

            if let Some(device) = IOHIDDevice::new(device as _) {
                vec.push(device);
            }
        }

        unsafe { CFRelease(devices as _) };

        vec
    }
}

impl Drop for IOHIDManager {
    fn drop(&mut self) {
        unsafe { CFRelease(self.as_CFTypeRef()) }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct IOHIDDevice(IOHIDDeviceRef);

impl_TCFType!(IOHIDDevice, IOHIDDeviceRef, IOHIDDeviceGetTypeID);

impl IOHIDDevice {
    pub fn new(device: IOHIDDeviceRef) -> Option<IOHIDDevice> {
        if device.is_null() {
            None
        } else {
            Some(IOHIDDevice(device))
        }
    }

    pub fn get_name(&self) -> Option<String> {
        match self.get_string_property(kIOHIDProductKey) {
            Some(name) => Some(name.to_string()),
            None => None,
        }
    }

    pub fn get_location_id(&self) -> Option<u32> {
        match self.get_number_property(kIOHIDLocationIDKey) {
            Some(location_id) => match location_id.to_i32() {
                Some(location_id) => Some(location_id as u32),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_bustype(&self) -> Option<u16> {
        match self.get_transport_key() {
            Some(transport_key) => {
                if transport_key == "USB".to_string() {
                    Some(0x03)
                } else if transport_key == "Bluetooth".to_string() {
                    Some(0x05)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub fn get_transport_key(&self) -> Option<String> {
        match self.get_string_property(kIOHIDTransportKey) {
            Some(transport_key) => Some(transport_key.to_string()),
            None => None,
        }
    }

    pub fn get_vendor_id(&self) -> Option<u16> {
        match self.get_number_property(kIOHIDVendorIDKey) {
            Some(vendor_id) => match vendor_id.to_i32() {
                Some(vendor_id) => Some(vendor_id as u16),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_product_id(&self) -> Option<u16> {
        match self.get_number_property(kIOHIDProductIDKey) {
            Some(product_id) => match product_id.to_i32() {
                Some(product_id) => Some(product_id as u16),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_version(&self) -> Option<u16> {
        match self.get_number_property(kIOHIDVersionNumberKey) {
            Some(version) => match version.to_i32() {
                Some(version) => Some(version as u16),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_page(&self) -> Option<u32> {
        match self.get_number_property(kIOHIDPrimaryUsagePageKey) {
            Some(page) => match page.to_i32() {
                Some(page) => Some(page as u32),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_usage(&self) -> Option<u32> {
        match self.get_number_property(kIOHIDPrimaryUsageKey) {
            Some(usage) => match usage.to_i32() {
                Some(usage) => Some(usage as u32),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_service(&self) -> Option<IOService> {
        unsafe { IOService::new(IOHIDDeviceGetService(self.0)) }
    }

    pub fn get_elements(&self) -> Vec<IOHIDElement> {
        let elements =
            unsafe { IOHIDDeviceCopyMatchingElements(self.0, ptr::null(), kIOHIDOptionsTypeNone) };

        if elements.is_null() {
            return vec![];
        }

        let element_count = unsafe { CFArrayGetCount(elements) };
        let mut vec = Vec::with_capacity(element_count as _);

        for i in 0..element_count {
            let element = unsafe { CFArrayGetValueAtIndex(elements, i) };

            if element.is_null() {
                continue;
            }

            vec.push(IOHIDElement(element as _));
        }

        vec
    }
}

impl Properties for IOHIDDevice {
    fn get_property(&self, key: *const c_char) -> Option<CFType> {
        let key =
            unsafe { CFStringCreateWithCString(kCFAllocatorDefault, key, kCFStringEncodingUTF8) };
        let value = unsafe { IOHIDDeviceGetProperty(self.0, key) };

        if value.is_null() {
            None
        } else {
            Some(unsafe { TCFType::wrap_under_get_rule(value) })
        }
    }
}

unsafe impl Send for IOHIDDevice {}
unsafe impl Sync for IOHIDDevice {}

#[repr(C)]
#[derive(Debug)]
pub struct IOHIDElement(IOHIDElementRef);

impl_TCFType!(IOHIDElement, IOHIDElementRef, IOHIDElementGetTypeID);

impl IOHIDElement {
    pub fn is_collection_type(type_: u32) -> bool {
        type_ == kIOHIDElementTypeCollection
    }

    pub fn is_axis(type_: u32, page: u32, usage: u32) -> bool {
        match type_ {
            kIOHIDElementTypeInput_Misc
            | kIOHIDElementTypeInput_Button
            | kIOHIDElementTypeInput_Axis => match page {
                kHIDPage_GenericDesktop => match usage {
                    kHIDUsage_GD_X | kHIDUsage_GD_Y | kHIDUsage_GD_Z | kHIDUsage_GD_Rx
                    | kHIDUsage_GD_Ry | kHIDUsage_GD_Rz | kHIDUsage_GD_Slider
                    | kHIDUsage_GD_Dial | kHIDUsage_GD_Wheel => true,
                    _ => false,
                },
                kHIDPage_Simulation => match usage {
                    kHIDUsage_Sim_Rudder
                    | kHIDUsage_Sim_Throttle
                    | kHIDUsage_Sim_Accelerator
                    | kHIDUsage_Sim_Brake => true,
                    _ => false,
                },
                _ => false,
            },
            _ => false,
        }
    }

    pub fn is_button(type_: u32, page: u32, usage: u32) -> bool {
        match type_ {
            kIOHIDElementTypeInput_Misc
            | kIOHIDElementTypeInput_Button
            | kIOHIDElementTypeInput_Axis => match page {
                kHIDPage_GenericDesktop => match usage {
                    kHIDUsage_GD_DPadUp
                    | kHIDUsage_GD_DPadDown
                    | kHIDUsage_GD_DPadRight
                    | kHIDUsage_GD_DPadLeft
                    | kHIDUsage_GD_Start
                    | kHIDUsage_GD_Select
                    | kHIDUsage_GD_SystemMainMenu => true,
                    _ => false,
                },
                kHIDPage_Button | kHIDPage_Consumer => true,
                _ => false,
            },
            _ => false,
        }
    }

    pub fn is_hat(type_: u32, page: u32, usage: u32) -> bool {
        match type_ {
            kIOHIDElementTypeInput_Misc
            | kIOHIDElementTypeInput_Button
            | kIOHIDElementTypeInput_Axis => match page {
                kHIDPage_GenericDesktop => match usage {
                    USAGE_AXIS_DPADX => true,
                    USAGE_AXIS_DPADY => true,
                    _ => false,
                },
                _ => false,
            },
            _ => false,
        }
    }

    pub fn get_cookie(&self) -> u32 {
        unsafe { IOHIDElementGetCookie(self.0) }
    }

    pub fn get_type(&self) -> u32 {
        unsafe { IOHIDElementGetType(self.0) }
    }

    pub fn get_page(&self) -> u32 {
        unsafe { IOHIDElementGetUsagePage(self.0) }
    }

    pub fn get_usage(&self) -> u32 {
        unsafe { IOHIDElementGetUsage(self.0) }
    }

    pub fn get_logical_min(&self) -> i64 {
        unsafe { IOHIDElementGetLogicalMin(self.0) }
    }

    pub fn get_logical_max(&self) -> i64 {
        unsafe { IOHIDElementGetLogicalMax(self.0) }
    }

    pub fn get_calibration_dead_zone_min(&self) -> Option<i64> {
        match self.get_number_property(kIOHIDElementCalibrationDeadZoneMinKey) {
            Some(calibration_dead_zone_min) => calibration_dead_zone_min.to_i64(),
            None => None,
        }
    }

    pub fn get_calibration_dead_zone_max(&self) -> Option<i64> {
        match self.get_number_property(kIOHIDElementCalibrationDeadZoneMaxKey) {
            Some(calibration_dead_zone_max) => calibration_dead_zone_max.to_i64(),
            None => None,
        }
    }

    pub fn get_children(&self) -> Vec<IOHIDElement> {
        let elements = unsafe { IOHIDElementGetChildren(self.0) };

        if elements.is_null() {
            return vec![];
        }

        let element_count = unsafe { CFArrayGetCount(elements) };
        let mut vec = Vec::with_capacity(element_count as _);

        for i in 0..element_count {
            let element = unsafe { CFArrayGetValueAtIndex(elements, i) };

            if element.is_null() {
                continue;
            }

            vec.push(IOHIDElement(element as _));
        }

        vec
    }
}

impl Properties for IOHIDElement {
    fn get_property(&self, key: *const c_char) -> Option<CFType> {
        let key =
            unsafe { CFStringCreateWithCString(kCFAllocatorDefault, key, kCFStringEncodingUTF8) };
        let value = unsafe { IOHIDElementGetProperty(self.0, key) };

        if value.is_null() {
            None
        } else {
            Some(unsafe { TCFType::wrap_under_get_rule(value) })
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct IOHIDValue(IOHIDValueRef);

impl_TCFType!(IOHIDValue, IOHIDValueRef, IOHIDValueGetTypeID);

impl IOHIDValue {
    pub fn new(value: IOHIDValueRef) -> Option<IOHIDValue> {
        if value.is_null() {
            None
        } else {
            Some(IOHIDValue(value))
        }
    }

    pub fn get_value(&self) -> i64 {
        unsafe { IOHIDValueGetIntegerValue(self.0) }
    }

    pub fn get_element(&self) -> Option<IOHIDElement> {
        let element = unsafe { IOHIDValueGetElement(self.0) };

        if element.is_null() {
            None
        } else {
            Some(IOHIDElement(element))
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct IOService(io_service_t);

impl IOService {
    pub fn new(io_service: io_service_t) -> Option<IOService> {
        if io_service == IO_OBJECT_NULL {
            return None;
        }

        let result = unsafe { IOObjectRetain(io_service) };

        if result == kIOReturnSuccess {
            Some(IOService(io_service))
        } else {
            None
        }
    }

    pub fn get_registry_entry_id(&self) -> Option<u64> {
        unsafe {
            IOObjectRetain(self.0);

            let mut entry_id = 0;
            let result = IORegistryEntryGetRegistryEntryID(self.0, &mut entry_id);

            IOObjectRelease(self.0);

            if result == kIOReturnSuccess {
                Some(entry_id)
            } else {
                None
            }
        }
    }
}

impl Drop for IOService {
    fn drop(&mut self) {
        unsafe {
            IOObjectRelease(self.0 as _);
        }
    }
}

trait Properties {
    fn get_number_property(&self, key: *const c_char) -> Option<CFNumber> {
        match self.get_property(key) {
            Some(value) => {
                if value.instance_of::<CFNumber>() {
                    Some(unsafe { CFNumber::wrap_under_get_rule(value.as_CFTypeRef() as _) })
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn get_string_property(&self, key: *const c_char) -> Option<CFString> {
        match self.get_property(key) {
            Some(value) => {
                if value.instance_of::<CFString>() {
                    Some(unsafe { CFString::wrap_under_get_rule(value.as_CFTypeRef() as _) })
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn get_property(&self, key: *const c_char) -> Option<CFType>;
}

fn create_hid_device_matcher(page: u32, usage: u32) -> CFDictionary<CFString, CFNumber> {
    let page_key = unsafe { CStr::from_ptr(kIOHIDDeviceUsagePageKey as _) };
    let page_key = CFString::from(page_key.to_str().unwrap());
    let page_value = CFNumber::from(page as i32);

    let usage_key = unsafe { CStr::from_ptr(kIOHIDDeviceUsageKey as _) };
    let usage_key = CFString::from(usage_key.to_str().unwrap());
    let usage_value = CFNumber::from(usage as i32);

    CFDictionary::from_CFType_pairs(&[(page_key, page_value), (usage_key, usage_value)])
}

extern "C" fn cf_set_applier(value: *const c_void, context: *const c_void) {
    unsafe { CFArrayAppendValue(context as _, value) };
}

// Usage Pages
pub const PAGE_GENERIC_DESKTOP: u32 = kHIDPage_GenericDesktop;
pub const PAGE_BUTTON: u32 = kHIDPage_Button;

// GenericDesktop Page (0x01)
pub const USAGE_AXIS_LSTICKX: u32 = kHIDUsage_GD_X;
pub const USAGE_AXIS_LSTICKY: u32 = kHIDUsage_GD_Y;
#[allow(dead_code)]
pub const USAGE_AXIS_LEFTZ: u32 = 0;
pub const USAGE_AXIS_RSTICKX: u32 = kHIDUsage_GD_Rx;
pub const USAGE_AXIS_RSTICKY: u32 = kHIDUsage_GD_Ry;
#[allow(dead_code)]
pub const USAGE_AXIS_RIGHTZ: u32 = 0;
pub const USAGE_AXIS_DPADX: u32 = kHIDUsage_GD_Hatswitch;
pub const USAGE_AXIS_DPADY: u32 = kHIDUsage_GD_Hatswitch + 1; // This "+ 1" is assumed and hard-coded elsewhere
#[allow(dead_code)]
pub const USAGE_AXIS_RT: u32 = 0;
#[allow(dead_code)]
pub const USAGE_AXIS_LT: u32 = 0;
pub const USAGE_AXIS_RT2: u32 = kHIDUsage_GD_Z;
pub const USAGE_AXIS_LT2: u32 = kHIDUsage_GD_Rz;

// Button Page (0x09)
pub const USAGE_BTN_SOUTH: u32 = kHIDUsage_Button_1;
pub const USAGE_BTN_EAST: u32 = kHIDUsage_Button_1 + 1;
pub const USAGE_BTN_NORTH: u32 = kHIDUsage_Button_1 + 2;
pub const USAGE_BTN_WEST: u32 = kHIDUsage_Button_1 + 3;
pub const USAGE_BTN_LT: u32 = kHIDUsage_Button_1 + 4;
pub const USAGE_BTN_RT: u32 = kHIDUsage_Button_1 + 5;
pub const USAGE_BTN_LT2: u32 = kHIDUsage_Button_1 + 6;
pub const USAGE_BTN_RT2: u32 = kHIDUsage_Button_1 + 7;
pub const USAGE_BTN_START: u32 = kHIDUsage_Button_1 + 8;
pub const USAGE_BTN_SELECT: u32 = kHIDUsage_Button_1 + 9;
pub const USAGE_BTN_MODE: u32 = kHIDUsage_Button_1 + 10;
pub const USAGE_BTN_DPAD_UP: u32 = kHIDUsage_Button_1 + 11;
pub const USAGE_BTN_DPAD_DOWN: u32 = kHIDUsage_Button_1 + 12;
pub const USAGE_BTN_DPAD_LEFT: u32 = kHIDUsage_Button_1 + 13;
pub const USAGE_BTN_DPAD_RIGHT: u32 = kHIDUsage_Button_1 + 14;
#[allow(dead_code)]
pub const USAGE_BTN_C: u32 = kHIDUsage_Button_1 + 15;
#[allow(dead_code)]
pub const USAGE_BTN_Z: u32 = kHIDUsage_Button_1 + 16;
#[allow(dead_code)]
pub const USAGE_BTN_LTHUMB: u32 = kHIDUsage_Button_1 + 17;
#[allow(dead_code)]
pub const USAGE_BTN_RTHUMB: u32 = kHIDUsage_Button_1 + 18;
