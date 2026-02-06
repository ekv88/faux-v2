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
      .with_inner_size([315.0, 430.0])
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
          self.hotkey_capture = None;
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

          let group_width = (ui.available_width() - 18.0).max(0.0);
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
              + 5.0;
            let response = ui.add(
              egui::TextEdit::singleline(&mut self.config.api_key)
                .hint_text("JWT / API token")
                .password(true)
                .font(egui::FontId::proportional(font_size))
                .desired_width((ui.available_width() - 5.0).max(0.0)),
            );
            changed |= response.changed();
          });

          ui.add_space(10.0);

          let group_width = (ui.available_width() - 23.0).max(0.0);
          group_frame.show(ui, |ui| {
            ui.set_min_width(group_width);
            ui.set_max_width(group_width);
            ui.label(egui::RichText::new("Visibility").strong());
            ui.add_space(6.0);
            egui::Grid::new("visibility_grid")
              .num_columns(2)
              .spacing([8.0, 6.0])
              .show(ui, |ui| {
                ui.label("Window opacity");
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

                ui.label("Max response width");
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

                ui.label("Max response height");
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

          let total_width = ui.available_width();
          let gap = 5.0;
          let colors_width = 100.0_f32.min(total_width);
          let hotkeys_width = 140.0_f32.min((total_width - gap - colors_width).max(0.0));

          ui.horizontal(|ui| {
            group_frame.show(ui, |ui| {
              ui.set_min_width(colors_width);
              ui.set_max_width(colors_width);
              let inner = egui::Frame::none().inner_margin(egui::Margin {
                left: 0.0,
                right: -10.0,
                top: 0.0,
                bottom: 0.0,
              });
              inner.show(ui, |ui| {
                ui.vertical(|ui| {
                  ui.label(egui::RichText::new("Colors & Theme").strong());
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
                  ui.add_space(6.0);
                  let mut theme = self.config.theme.clone();
                  egui::ComboBox::from_id_source("theme_select")
                    .selected_text(theme.clone())
                    .width(100.0)
                    .show_ui(ui, |ui| {
                      ui.selectable_value(&mut theme, "dark".to_string(), "Dark");
                      ui.selectable_value(&mut theme, "light".to_string(), "Light");
                    });
                  if theme != self.config.theme {
                    self.config.theme = theme;
                    self.save_config();
                  }
                });
              });
            });

            ui.add_space(gap);

            group_frame.show(ui, |ui| {
              ui.set_min_width(hotkeys_width);
              ui.set_max_width(hotkeys_width);
              let inner = egui::Frame::none().inner_margin(egui::Margin {
                left: 0.0,
                right: -10.0,
                top: 0.0,
                bottom: 0.0,
              });
              inner.show(ui, |ui| {
                ui.vertical(|ui| {
                  ui.label(egui::RichText::new("Hotkeys").strong());
                  ui.add_space(6.0);
                  egui::Grid::new("hotkey_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                      ui.label("Screenshot");
                      ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                        self.modifiers_row(ui, 12.0);
                        ui.label("+");
                        let label = if self.hotkey_capture == Some(super::HotkeyAction::Screenshot) {
                          "Press key...".to_string()
                        } else {
                          Self::hotkey_label_from_token(&self.config.hotkeys.screenshot)
                        };
                        if self.text_badge(ui, &label, 3.0, 2.0, true).clicked() {
                          self.hotkey_capture = Some(super::HotkeyAction::Screenshot);
                        }
                      });
                      ui.end_row();

                      ui.label("Close resp.");
                      ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                        self.modifiers_row(ui, 12.0);
                        ui.label("+");
                        let label = if self.hotkey_capture == Some(super::HotkeyAction::CloseResponse) {
                          "Press key...".to_string()
                        } else {
                          Self::hotkey_label_from_token(&self.config.hotkeys.close_response)
                        };
                        if self.text_badge(ui, &label, 3.0, 2.0, true).clicked() {
                          self.hotkey_capture = Some(super::HotkeyAction::CloseResponse);
                        }
                      });
                      ui.end_row();

                      ui.label("Show/Hide");
                      ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                        self.modifiers_row(ui, 12.0);
                        ui.label("+");
                        let label = if self.hotkey_capture == Some(super::HotkeyAction::ShowHide) {
                          "Press key...".to_string()
                        } else {
                          Self::hotkey_label_from_token(&self.config.hotkeys.show_hide)
                        };
                        if self.text_badge(ui, &label, 3.0, 2.0, true).clicked() {
                          self.hotkey_capture = Some(super::HotkeyAction::ShowHide);
                        }
                      });
                      ui.end_row();

                      ui.label("Quit app");
                      ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                        self.modifiers_row(ui, 12.0);
                        ui.label("+");
                        let label = if self.hotkey_capture == Some(super::HotkeyAction::Quit) {
                          "Press key...".to_string()
                        } else {
                          Self::hotkey_label_from_token(&self.config.hotkeys.quit)
                        };
                        if self.text_badge(ui, &label, 3.0, 2.0, true).clicked() {
                          self.hotkey_capture = Some(super::HotkeyAction::Quit);
                        }
                      });
                      ui.end_row();
                    });
                });
              });
            });
          });

          ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.add_space(5.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
              ui.add_space(0.0);
              let close_text = egui::RichText::new("Close").size(14.3);
              if ui.button(close_text).clicked() {
                self.settings_open = false;
                self.settings_hwnd_hooked = false;
                self.hotkey_capture = None;
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
                self.schedule_config_save();
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
              ui.visuals_mut().override_text_color = None;
              let mut color = self.config.text_color.to_color32();
              let changed_picker = egui::color_picker::color_picker_color32(
                ui,
                &mut color,
                egui::color_picker::Alpha::Opaque,
              );
              if changed_picker {
                let color = egui::Color32::from_rgb(color.r(), color.g(), color.b());
                self.config.text_color = ColorConfig::from_color32(color);
                self.schedule_config_save();
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
                self.schedule_config_save();
              }
              if ui.button("Close").clicked() {
                self.divider_picker_open = false;
              }
            });
        }

        self.flush_config_if_needed();
      },
    );
  }
}
