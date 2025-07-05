#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::*;
use mech::*;
use mech_notebook::*;
use std::sync::Arc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> eframe::Result {
  let icon = icon::load_icon();

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]).with_icon(Arc::new(icon)),
    ..Default::default()
  };

  let mut input = String::new();
  let mut terminal_output = String::new();
  let mut text_edit_focus_id = egui::Id::new("terminal_input");

  let id = hash_str("mech-notebook");
  let mut intrp = Interpreter::new(id);
  let mut repl = MechRepl::from(intrp);

  let mut scroll_to_bottom = false;
  terminal_output.push_str(&format!("Mech v{}\n",VERSION));
  eframe::run_simple_native("Mech Terminal", options, move |ctx, _frame| {

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(24,0,14);
    visuals.extreme_bg_color = Color32::from_rgb(24,0,14);
    ctx.set_visuals(visuals);


    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("FiraCode-Regular".to_owned(),Arc::new(FontData::from_static(include_bytes!("../../fonts/FiraCode-Regular.ttf"))));
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "FiraCode-Regular".to_owned());
    ctx.set_fonts(fonts);

    let screen_rect = ctx.screen_rect();
    let window_size = screen_rect.height();

    egui::CentralPanel::default().show(ctx, |ui| {
      egui::ScrollArea::vertical()
        .max_height(window_size - 50.0)
        .stick_to_bottom(true)
        .animated(false)
        .show(ui, |ui| {
          ui.label(&terminal_output);
          if scroll_to_bottom {
            ui.scroll_to_cursor(Some(Align::BOTTOM));
            scroll_to_bottom = false;
          }
        });
      ui.horizontal(|ui| {
        ui.label(">:");
        let response = ui.add(
          egui::TextEdit::singleline(&mut input)
            .id(text_edit_focus_id)
            .frame(false)
        );
        if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
          terminal_output.push_str(&format!(">: {}\n", input));
          if input.chars().nth(0) == Some(':') {
            match parse_repl_command(&input.as_str()) {
              Ok((_, repl_command)) => {
                match repl.execute_repl_command(repl_command) {
                  Ok(output) => {
                    terminal_output.push_str(&format!("{}\n", output));
                  }
                  Err(err) => {
                    terminal_output.push_str(&format!("{:?}\n", err));
                  }
                }
              }
              _ => todo!(),
            }
          } else if input.trim() == "" {
            //continue;
          } else {
            let cmd = ReplCommand::Code(vec![("repl".to_string(),MechSourceCode::String(input.clone()))]);
            match repl.execute_repl_command(cmd) {
              Ok(output) => {
                terminal_output.push_str(&format!("{}\n", output));
              }
              Err(err) => {
                terminal_output.push_str(&format!("{:?}\n", err));
              }
            }
          }
          input.clear();
        }
        response.request_focus();
      });
    });
  })
}
