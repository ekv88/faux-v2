use eframe::egui;
use egui_commonmark::CommonMarkViewer;

use crate::ui::show_skeleton;

use super::AppState;

impl AppState {
  fn measure_max_line_width(
    ctx: &egui::Context,
    text: &str,
    font_id: egui::FontId,
  ) -> f32 {
    let mut max_width: f32 = 0.0;
    for line in text.lines() {
      let galley =
        ctx.fonts(|fonts| fonts.layout_no_wrap(line.to_owned(), font_id.clone(), egui::Color32::WHITE));
      max_width = max_width.max(galley.size().x);
    }
    max_width
  }

  fn desired_response_width(&self, ctx: &egui::Context) -> f32 {
    let body_font = egui::TextStyle::Body.resolve(&ctx.style());
    let mut desired_width = Self::RESPONSE_MIN_WIDTH;
    if let Some(err) = &self.last_error {
      let err_width = Self::measure_max_line_width(ctx, err, body_font);
      desired_width = err_width + 36.0;
    } else if let Some(response) = &self.response {
      let text_width = Self::measure_max_line_width(ctx, &response.text, body_font.clone());
      let code_font = egui::FontId::monospace(12.0);
      let code_width = Self::measure_max_line_width(ctx, &response.code, code_font);
      let left_share = 0.48_f32;
      let right_share = 1.0 - left_share;
      let spacing = ctx.style().spacing.item_spacing.x;
      let divider_width = 1.0;
      let left_required = text_width.max(140.0) / left_share;
      let right_required = (code_width.max(180.0) + divider_width + spacing * 2.0) / right_share;
      let content_width = left_required.max(right_required);
      desired_width = content_width + 28.0;
    }
    let max_width = self.config.response_max_width.max(Self::RESPONSE_MIN_WIDTH);
    desired_width
      .clamp(Self::RESPONSE_MIN_WIDTH, max_width)
      .ceil()
  }

  #[cfg(target_os = "windows")]
  fn apply_windows_response_transparency(&mut self) {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::COLORREF;
    use windows::Win32::UI::WindowsAndMessaging::{
      FindWindowW, GetWindowLongW, SetLayeredWindowAttributes, SetWindowDisplayAffinity,
      SetWindowLongW, GWL_EXSTYLE, LWA_ALPHA, WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WS_EX_APPWINDOW,
      WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    };

    let title: Vec<u16> = Self::RESPONSE_TITLE
      .encode_utf16()
      .chain(std::iter::once(0))
      .collect();
    let hwnd = unsafe { FindWindowW(None, PCWSTR::from_raw(title.as_ptr())) };
    if hwnd.0 == 0 {
      return;
    }

    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
    let mut new_style = ex_style | WS_EX_LAYERED.0 as i32 | WS_EX_TRANSPARENT.0 as i32;
    new_style |= WS_EX_TOOLWINDOW.0 as i32;
    new_style &= !(WS_EX_APPWINDOW.0 as i32);
    let alpha =
      (self.config.opacity.clamp(0.1, 1.0) * 255.0).round().clamp(0.0, 255.0) as u8;
    unsafe {
      let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
      let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
      let _ = SetWindowDisplayAffinity(
        hwnd,
        if self.config.stealth { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE },
      );
    }
    self.response_hwnd_hooked = true;
  }

  pub(super) fn show_response_window(&mut self, ctx: &egui::Context) {
    if !self.response_open {
      return;
    }

    let Some(main_rect) = ctx.input(|i| i.viewport().outer_rect) else {
      return;
    };
    let anchor_pos = egui::pos2(main_rect.min.x, main_rect.max.y + Self::RESPONSE_ANCHOR_GAP);

    let desired_width = self.desired_response_width(ctx);
    let mut width_changed = false;
    if (self.response_size.x - desired_width).abs() > 1.0 {
      self.response_size.x = desired_width;
      width_changed = true;
    }
    let max_height = self.config.response_max_height.max(120.0);
    let mut height_changed = false;
    if self.response.is_some() && !self.loading && self.last_error.is_none() {
      if (self.response_size.y - max_height).abs() > 1.0 {
        self.response_size.y = max_height;
        height_changed = true;
      }
    }
    if self.response_size.y > max_height {
      self.response_size.y = max_height;
      height_changed = true;
    }

    let viewport = egui::ViewportBuilder::default()
      .with_title(Self::RESPONSE_TITLE)
      .with_inner_size([self.response_size.x, self.response_size.y])
      .with_position(anchor_pos)
      .with_resizable(false)
      .with_decorations(false)
      .with_transparent(true)
      .with_taskbar(false)
      .with_mouse_passthrough(true);
    let viewport = if self.config.always_on_top {
      viewport.with_always_on_top()
    } else {
      viewport
    };

    ctx.show_viewport_immediate(
      egui::ViewportId::from_hash_of("response"),
      viewport,
      |ctx, _class| {
        if ctx.input(|i| i.viewport().close_requested()) {
          self.close_response();
          ctx.send_viewport_cmd(egui::ViewportCommand::Close);
          return;
        }

        if self
          .response_last_pos
          .map_or(true, |prev| (prev - anchor_pos).length_sq() > 0.5)
        {
          ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(anchor_pos));
          self.response_last_pos = Some(anchor_pos);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Transparent(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.config.always_on_top {
          egui::WindowLevel::AlwaysOnTop
        } else {
          egui::WindowLevel::Normal
        }));
        if width_changed || height_changed {
          ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            desired_width,
            self.response_size.y,
          )));
        }

        #[cfg(target_os = "windows")]
        self.apply_windows_response_transparency();

        let panel_frame = egui::Frame::none()
          .fill(egui::Color32::TRANSPARENT)
          .stroke(egui::Stroke::NONE);

        let fill = if cfg!(target_os = "windows") {
          self.background_color_layered()
        } else {
          self.background_color()
        };
        let frame = egui::Frame::none()
          .fill(fill)
          .stroke(egui::Stroke::new(1.0, self.border_color()))
          .rounding(egui::Rounding::same(5.0))
          .inner_margin(egui::Margin::symmetric(14.0, 12.0));

        let mut content_size = None;
        egui::CentralPanel::default()
          .frame(panel_frame)
          .show(ctx, |ui| {
            ui.visuals_mut().panel_fill = egui::Color32::TRANSPARENT;
            ui.visuals_mut().window_fill = egui::Color32::TRANSPARENT;
            ui.visuals_mut().override_text_color = Some(self.text_color());

            ui.set_width_range(desired_width..=desired_width);
            let response = frame.show(ui, |ui| {
              ui.visuals_mut().override_text_color = Some(self.text_color());
              ui.horizontal(|ui| {
                let icon_size = 14.0;
                self.modifiers_row(ui, icon_size);
                let close_label = Self::hotkey_label_from_token(&self.config.hotkeys.close_response);
                ui.label(format!("+ {}", close_label));
                ui.label("Close response");
              });
              ui.add_space(8.0);
              ui.separator();
              ui.add_space(8.0);

              if self.loading {
                show_skeleton(ui, self.skeleton_color());
                return;
              }

              if let Some(err) = &self.last_error {
                ui.add_space(5.0);
                let error_frame = egui::Frame::none()
                  .fill(egui::Color32::from_rgba_unmultiplied(120, 32, 32, 200))
                  .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(150, 60, 60, 220),
                  ))
                  .rounding(egui::Rounding::same(6.0))
                  .inner_margin(egui::Margin::same(10.0));
                let max_width = (ui.available_width() - 20.0).max(0.0);
                error_frame.show(ui, |ui| {
                  ui.set_max_width(max_width);
                  ui.horizontal(|ui| {
                    let icon_frame = egui::Frame::none()
                      .fill(egui::Color32::from_rgba_unmultiplied(170, 55, 55, 220))
                      .rounding(egui::Rounding::same(8.0))
                      .inner_margin(egui::Margin::symmetric(6.0, 2.0));
                    icon_frame.show(ui, |ui| {
                      ui.label(
                        egui::RichText::new("!")
                          .strong()
                          .color(egui::Color32::from_rgb(255, 225, 225)),
                      );
                    });
                    ui.add_space(6.0);
                    ui.add(
                      egui::Label::new(
                        egui::RichText::new(err)
                          .color(egui::Color32::from_rgb(255, 220, 220)),
                      )
                      .wrap(true),
                    );
                  });
                });
                ui.add_space(10.0);
                return;
              }

              if let Some(response) = &self.response {
                let response_text = response.text.clone();
                let text_color = self.text_color();
                let output = egui::ScrollArea::vertical()
                  .id_source("response_scroll")
                  .auto_shrink([false, true])
                  .scroll_offset(egui::vec2(0.0, self.response_scroll_offset))
                  .show(ui, |ui| {
                    ui.scope(|ui| {
                      ui.visuals_mut().override_text_color = None;
                      ui.visuals_mut().widgets.noninteractive.fg_stroke.color = text_color;
                      let viewer = CommonMarkViewer::new("response_markdown")
                        .syntax_theme_dark("base16-ocean.dark")
                        .syntax_theme_light("base16-ocean.light");
                      viewer.show(ui, &mut self.markdown_cache, &response_text);
                    });
                  });
                let max_offset = (output.content_size.y - output.inner_rect.height()).max(0.0);
                if self.response_scroll_offset > max_offset {
                  self.response_scroll_offset = max_offset;
                }
                self.response_scroll_max = max_offset;
              }
            });
            let frame_rect = response.response.rect;
            self.show_response_status_overlay(ctx, frame_rect);
            content_size = Some(frame_rect.size());
          });

        if let Some(size) = content_size {
          let max_height = self.config.response_max_height.max(120.0);
          let desired_height = size.y.clamp(120.0, max_height).ceil();
          if (self.response_size.y - desired_height).abs() > 1.0 {
            self.response_size.y = desired_height;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
              self.response_size.x,
              desired_height,
            )));
          }
        }
      },
    );
  }

  fn show_response_status_overlay(&mut self, ctx: &egui::Context, frame_rect: egui::Rect) {
    if !self.loading && self.response_status.is_none() {
      return;
    }

    let font_id = egui::FontId::proportional(11.0);
    let text_galley = self.response_status.as_ref().map(|status| {
      ctx.fonts(|fonts| fonts.layout_no_wrap(status.to_owned(), font_id.clone(), egui::Color32::WHITE))
    });
    let text_size = text_galley.as_ref().map(|g| g.size()).unwrap_or(egui::vec2(0.0, 0.0));
    let spinner_size = if self.loading { 12.0 } else { 0.0 };
    let gap = if self.loading && self.response_status.is_some() { 6.0 } else { 0.0 };
    let padding = egui::vec2(8.0, 4.0);
    let width = padding.x * 2.0 + spinner_size + gap + text_size.x;
    let height = padding.y * 2.0 + text_size.y.max(spinner_size);

    let pos = egui::pos2(frame_rect.right() - width - 8.0, frame_rect.top() + 8.0);
    egui::Area::new(egui::Id::new("response_status_overlay"))
      .order(egui::Order::Foreground)
      .fixed_pos(pos)
      .show(ctx, |ui| {
        ui.visuals_mut().override_text_color = Some(self.text_color());
        ui.set_min_size(egui::vec2(width, height));
        let frame = egui::Frame::none()
          .fill(self.button_fill(false))
          .stroke(egui::Stroke::new(1.0, self.button_border()))
          .rounding(egui::Rounding::same(6.0))
          .inner_margin(egui::Margin::symmetric(padding.x, padding.y));
        frame.show(ui, |ui| {
          ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
            if self.loading {
              ui.add(egui::Spinner::new().size(12.0));
            }
            if let Some(status) = &self.response_status {
              ui.label(egui::RichText::new(status).size(11.0));
            }
          });
        });
      });
  }
}
