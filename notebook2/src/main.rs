#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::{epi, egui};
use eframe::egui::{containers::*, *};
extern crate mech;

use mech::program::*;
use mech::utilities::*;
use mech::core::*;
use std::thread::JoinHandle;

struct MechApp {
  mech_client: RunLoop,
  ticks: u64,
  maestro_thread: Option<JoinHandle<()>>,
}

impl Default for MechApp {
    fn default() -> Self {
      let runner = ProgramRunner::new("Mech Run");
      let mech_client = runner.run().unwrap();
      let mcc = mech_client.outgoing.clone();

      let maestro_thread = match mech_client.socket_address {
        Some(ref mech_socket_address) => {
          Some(mech::start_maestro(mech_socket_address.to_string(), "127.0.0.1:0".to_string(), "127.0.0.1:3235".to_string(), "127.0.0.1:3235".to_string(), mcc).unwrap())
        }
        None => None,
      };
      mech_client.send(RunLoopMessage::Code(MechCode::String(r#"#x = #z + 1"#.to_string())));
      let thread_receiver = mech_client.incoming.clone();
      // Empty recv queue
      loop {
        match thread_receiver.recv().unwrap() {
          ClientMessage::Done => break,
          _ => (),
        }
      }
      Self {
        ticks: 0,
        mech_client,
        maestro_thread,
      }
    }
  }

impl epi::App for MechApp {

  fn name(&self) -> &str {
    "Mech Notebook"
  }

  fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
    let Self { mech_client, ticks, .. } = self;
    mech_client.send(RunLoopMessage::GetValue((hash_str("x"),TableIndex::Index(1),TableIndex::Index(1))));
    let thread_receiver = mech_client.incoming.clone();
    let mut values = vec![];
    let mut log = "".to_string();
    loop {
      match thread_receiver.recv().unwrap() {
        ClientMessage::Value(v) => values.push(v),
        ClientMessage::Done => break,
        x => {log=format!("{:?}\n{:?}",log,x)},
      }
    }
    if values.len() > 0 {
      match &values[0] {
        Value::U64(s) => {
          *ticks = (*s).into();
        }
        _ => (),
      } 
    }




    egui::CentralPanel::default().show(ctx, |ui| {

      let color = Color32::from_additive_luminance(196);

      Frame::dark_canvas(ui.style()).show(ui, |ui| {
        ui.ctx().request_repaint();
        let time = ui.input().time;

        let desired_size = ui.available_width() * vec2(1.0, 0.35);
        let (_id, rect) = ui.allocate_space(desired_size);

        let to_screen = emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);

        let mut shapes = vec![];

        for &mode in &[2] {
            let mode = mode as f64;
            let n = 120;
            let speed = 1.5;

            let points: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let t = i as f64 / (n as f64);
                    let amp = (*ticks as f64 / 16.0 * speed * mode).sin() / mode;
                    let y = amp * (t * std::f64::consts::TAU / 2.0 * mode).sin();
                    to_screen * pos2(t as f32, y as f32)
                })
                .collect();

            let thickness = 10.0 / mode as f32;
            shapes.push(epaint::Shape::line(points, Stroke::new(thickness, color)));
        }
        ui.painter().extend(shapes);
      });
      ui.heading(format!("{:?}", ticks));
    });
  }
}

fn main() {
    let app = MechApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
