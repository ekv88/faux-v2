use eframe::egui;
use egui_phosphor as phosphor;

pub fn install_phosphor_fonts(ctx: &egui::Context) {
  let mut fonts = egui::FontDefinitions::default();
  phosphor::add_to_fonts(&mut fonts, phosphor::Variant::Regular);
  ctx.set_fonts(fonts);
}

pub fn show_skeleton(ui: &mut egui::Ui, color: egui::Color32) {
  let rounding = egui::Rounding::same(4.0);
  for _ in 0..5 {
    let (rect, _) =
      ui.allocate_exact_size(egui::vec2(ui.available_width(), 12.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, rounding, color);
    ui.add_space(8.0);
  }
}

pub fn draw_vertical_divider(ui: &mut egui::Ui, height: f32, color: egui::Color32) {
  let height = height.max(12.0);
  let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, height), egui::Sense::hover());
  let stroke = egui::Stroke::new(1.0, color);
  let top = egui::pos2(rect.center().x, rect.top());
  let bottom = egui::pos2(rect.center().x, rect.bottom());
  ui.painter().line_segment([top, bottom], stroke);
}
