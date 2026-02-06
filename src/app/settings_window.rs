use eframe::egui;

use crate::config::ColorConfig;

use super::AppState;

impl AppState {
  pub(super) fn show_settings_window(&mut self, ctx: &egui::Context) {
    if !self.settings_open {
      return;
    }

    let viewport = egui::ViewportBuilder::default()
      .with_title("Settings")
      .with_inner_size([260.0, 405.0])
      .with_resizable(false)
      .with_transparent(true)
      .with_taskbar(false);
    let viewport = if self.config.always_on_top {
      viewport.with_always_on_top()
    } else {
      viewport
    };

    ctx.show_viewport_immediate(
      egui::ViewportId::from_hash_of("settings"),
      viewport,
      |ctx, _class| {
        if ctx.input(|i| i.viewport().close_requested()) {
          self.settings_open = false;
          self.settings_hwnd_hooked = false;
          ctx.send_viewport_cmd(egui::ViewportCommand::Close);
          return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
          if self.config.always_on_top {
            egui::WindowLevel::AlwaysOnTop
          } else {
            egui::WindowLevel::Normal
          },
        ));

        #[cfg(target_os = "windows")]
        {
          use windows::Win32::Foundation::HWND;
          if let Some(hwnd) = Self::find_window_by_title("Settings") {
            if !self.settings_hwnd_hooked {
              Self::apply_windows_tool_window(HWND(hwnd.0));
              self.settings_hwnd_hooked = true;
            }
            Self::apply_windows_exclude_from_capture(HWND(hwnd.0), self.config.stealth);
          }
        }

        let frame = egui::Frame::none()
          .fill(self.background_color())
          .stroke(egui::Stroke::new(1.0, self.border_color()))
          .rounding(egui::Rounding::same(0.0))
          .inner_margin(egui::Margin::symmetric(10.0, 10.0));

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
          ui.visuals_mut().override_text_color = Some(self.text_color());
          let mut changed = false;

          let group_frame = egui::Frame::none()
            .stroke(egui::Stroke::new(1.0, self.button_border()))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::same(10.0));

          let group_width = (ui.available_width() - 30.0).max(0.0);
          group_frame.show(ui, |ui| {
            ui.set_min_width(group_width);
            ui.label(egui::RichText::new("API Key").strong());
            ui.add_space(6.0);
            let font_size = ui
              .style()
              .text_styles
              .get(&egui::TextStyle::Body)
              .map(|style| style.size)
              .unwrap_or(14.0)
              + 2.0;
            let response = ui.add(
              egui::TextEdit::singleline(&mut self.config.api_key)
                .hint_text("JWT / API token")
                .password(true)
                .font(egui::FontId::proportional(font_size))
                .desired_width(ui.available_width()),
            );
            changed |= response.changed();
          });

          ui.add_space(10.0);

          let group_width = (ui.available_width() - 20.0).max(0.0);
          group_frame.show(ui, |ui| {
            ui.set_min_width(group_width);
            ui.label(egui::RichText::new("Visibility").strong());
            ui.add_space(6.0);
            egui::Grid::new("visibility_grid")
              .num_columns(2)
              .spacing([8.0, 6.0])
              .show(ui, |ui| {
                ui.label("Opacity");
                let mut opacity = self.config.opacity;
                let slider_width = (ui.available_width() - 15.0).max(0.0);
                if ui
                  .add_sized(
                    [slider_width, 18.0],
                    egui::Slider::new(&mut opacity, 0.2..=1.0).show_value(true),
                  )
                  .changed()
                {
                  self.config.opacity = opacity;
                  changed = true;
                }
                ui.end_row();

                ui.label("Max width");
                let mut max_width = self.config.response_max_width;
                let slider_width = (ui.available_width() - 15.0).max(0.0);
                if ui
                  .add_sized(
                    [slider_width, 18.0],
                    egui::Slider::new(&mut max_width, 320.0..=1600.0).show_value(true),
                  )
                  .changed()
                {
                  self.config.response_max_width = max_width;
                  changed = true;
                }
                ui.end_row();

                ui.label("Max height");
                let mut max_height = self.config.response_max_height;
                let slider_width = (ui.available_width() - 15.0).max(0.0);
                if ui
                  .add_sized(
                    [slider_width, 18.0],
                    egui::Slider::new(&mut max_height, 200.0..=1200.0).show_value(true),
                  )
                  .changed()
                {
                  self.config.response_max_height = max_height;
                  changed = true;
                }
                ui.end_row();
              });
            ui.add_space(6.0);
            changed |= ui
              .checkbox(&mut self.config.stealth, "Stealth (exclude from capture)")
              .changed();
            changed |= ui
              .checkbox(&mut self.config.always_on_top, "Always on top")
              .changed();
          });

          ui.add_space(10.0);

          let group_width = (ui.available_width() - 20.0).max(0.0);
          group_frame.show(ui, |ui| {
            ui.set_min_width(group_width);
            ui.label(egui::RichText::new("Colors").strong());
            ui.add_space(6.0);
            egui::Grid::new("color_grid")
              .num_columns(2)
              .spacing([8.0, 6.0])
              .show(ui, |ui| {
                ui.label("Background");
                if Self::color_swatch(ui, self.config.background.to_color32()).clicked() {
                  self.background_picker_open = !self.background_picker_open;
                  if self.background_picker_open {
                    self.text_picker_open = false;
                    self.divider_picker_open = false;
                  }
                }
                ui.end_row();

                ui.label("Text");
                if Self::color_swatch(ui, self.config.text_color.to_color32()).clicked() {
                  self.text_picker_open = !self.text_picker_open;
                  if self.text_picker_open {
                    self.background_picker_open = false;
                    self.divider_picker_open = false;
                  }
                }
                ui.end_row();

                ui.label("Divider");
                if Self::color_swatch(ui, self.config.divider_color.to_color32()).clicked() {
                  self.divider_picker_open = !self.divider_picker_open;
                  if self.divider_picker_open {
                    self.background_picker_open = false;
                    self.text_picker_open = false;
                  }
                }
                ui.end_row();
              });
          });

          ui.add_space(10.0);
          ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
              if ui.button("Close").clicked() {
                self.settings_open = false;
                self.settings_hwnd_hooked = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
              }
            });
          });

          if changed {
            self.save_config();
          }
        });

        if self.background_picker_open {
          egui::Window::new("Background Color")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
              let mut color = self.config.background.to_color32();
              let changed_picker = egui::color_picker::color_picker_color32(
                ui,
                &mut color,
                egui::color_picker::Alpha::OnlyBlend,
              );
              if changed_picker {
                self.config.background = ColorConfig::from_color32(color);
                self.save_config();
              }
              if ui.button("Close").clicked() {
                self.background_picker_open = false;
              }
            });
        }

        if self.text_picker_open {
          egui::Window::new("Text Color")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
              let mut color = self.config.text_color.to_color32();
              let changed_picker = egui::color_picker::color_picker_color32(
                ui,
                &mut color,
                egui::color_picker::Alpha::OnlyBlend,
              );
              if changed_picker {
                self.config.text_color = ColorConfig::from_color32(color);
                self.save_config();
              }
              if ui.button("Close").clicked() {
                self.text_picker_open = false;
              }
            });
        }

        if self.divider_picker_open {
          egui::Window::new("Divider Color")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
              let mut color = self.config.divider_color.to_color32();
              let changed_picker = egui::color_picker::color_picker_color32(
                ui,
                &mut color,
                egui::color_picker::Alpha::OnlyBlend,
              );
              if changed_picker {
                self.config.divider_color = ColorConfig::from_color32(color);
                self.save_config();
              }
              if ui.button("Close").clicked() {
                self.divider_picker_open = false;
              }
            });
        }
      },
    );
  }
}
