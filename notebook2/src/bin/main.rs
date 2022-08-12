#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit="256"]

use eframe::{egui};
use eframe::egui::{containers::*, *};
extern crate mech_gui;

use std::thread::JoinHandle;
extern crate image;
use std::path::Path;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;

use mech_gui::*;

fn main() {
    //let input = std::env::args().nth(1).unwrap();
    let mut native_options = eframe::NativeOptions::default();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/mech.ico");
    let icon = load_icon(Path::new(path));
    native_options.icon_data = Some(icon);
    native_options.min_window_size = Some(Vec2{x: 1480.0, y: 800.0});
    eframe::run_native("Mech Notebook", native_options, Box::new(|cc| Box::new(MechApp::new(cc))));
}


