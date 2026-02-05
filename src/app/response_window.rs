use eframe::egui;
use egui_commonmark::CommonMarkViewer;

use crate::ui::{draw_vertical_divider, show_skeleton};

use super::AppState;

impl AppState {
  const FORCE_SAMPLE_CODE: bool = true;
  const SAMPLE_CODE: &'static str = r###"#[derive(Debug)]
pub struct CodeExample {
    name: String,
    age: u32,
}

impl Default for CodeExample {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
        }
    }
}

impl CodeExample {
    fn samples_in_grid(&mut self, ui: &mut egui::Ui) {
        // Note: we keep the code narrow so that the example fits on a mobile screen.

        let Self { name, age } = self; // for brevity later on

        show_code(ui, r#"ui.heading("Example");"#);
        ui.heading("Example");
        ui.end_row();

        show_code(
            ui,
            r#"
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(name);
            });"#,
        );
        // Putting things on the same line using ui.horizontal:
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(name);
        });
        ui.end_row();

        show_code(
            ui,
            r#"
            ui.add(
                egui::DragValue::new(age)
                    .range(0..=120)
                    .suffix(" years"),
            );"#,
        );
        ui.add(egui::DragValue::new(age).range(0..=120).suffix(" years"));
        ui.end_row();

        show_code(
            ui,
            r#"
            if ui.button("Increment").clicked() {
                *age += 1;
            }"#,
        );
        if ui.button("Increment").clicked() {
            *age += 1;
        }
        ui.end_row();

        #[expect(clippy::literal_string_with_formatting_args)]
        show_code(ui, r#"ui.label(format!("{name} is {age}"));"#);
        ui.label(format!("{name} is {age}"));
        ui.end_row();
    }

    fn code(&mut self, ui: &mut egui::Ui) {
        show_code(
            ui,
            r"
pub struct CodeExample {
    name: String,
    age: u32,
}

impl CodeExample {
    fn ui(&mut self, ui: &mut egui::Ui) {
        // Saves us from writing `&mut self.name` etc
        let Self { name, age } = self;"#,
        );

        ui.horizontal(|ui| {
            let font_id = egui::TextStyle::Monospace.resolve(ui.style());
            let indentation = 2.0 * 4.0 * ui.fonts_mut(|f| f.glyph_width(&font_id, ' '));
            ui.add_space(indentation);

            egui::Grid::new("code_samples")
                .striped(true)
                .num_columns(2)
                .show(ui, |ui| {
                    self.samples_in_grid(ui);
                });
        });

        crate::rust_view_ui(ui, "    }\n}");
    }
}

impl crate::Demo for CodeExample {
    fn name(&self) -> &'static str {
        " Code Example"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        use crate::View as _;
        egui::Window::new(self.name())
            .open(open)
            .min_width(375.0)
            .default_size([390.0, 500.0])
            .scroll(false)
            .resizable([true, false]) // resizable so we can shrink if the text edit grows
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| self.ui(ui));
    }
}

impl crate::View for CodeExample {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
            self.code(ui);
        });

        ui.separator();

        crate::rust_view_ui(ui, &format!("{self:#?}"));

        ui.separator();

        let mut theme =
            egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
        ui.collapsing("Theme", |ui| {
            theme.ui(ui);
            theme.store_in_memory(ui.ctx());
        });

        ui.separator();

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}

fn show_code(ui: &mut egui::Ui, code: &str) {
    let code = remove_leading_indentation(code.trim_start_matches('\n'));
  crate::rust_view_ui(ui, &code);
}

fn remove_leading_indentation(code: &str) -> String {
  fn is_indent(c: &u8) -> bool {
    matches!(*c, b' ' | b'\t')
  }

  let first_line_indent = code.bytes().take_while(is_indent).count();

  let mut out = String::new();

  let mut code = code;
  while !code.is_empty() {
    let indent = code.bytes().take_while(is_indent).count();
    let start = first_line_indent.min(indent);
    let end = code
      .find('\n')
      .map_or_else(|| code.len(), |endline| endline + 1);
    out += &code[start..end];
    code = &code[end..];
    }
    out
}
"###;
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
    desired_width
      .clamp(Self::RESPONSE_MIN_WIDTH, Self::RESPONSE_MAX_WIDTH)
      .ceil()
  }

  fn code_with_line_numbers(ui: &mut egui::Ui, code: &str) {
    let mut lines: Vec<&str> = code.lines().collect();
    if lines.is_empty() {
      lines.push("");
    }
    let line_count = lines.len();
    let digits = line_count.to_string().len();
    let line_numbers = (1..=line_count)
      .map(|n| format!("{:>width$}", n, width = digits))
      .collect::<Vec<_>>()
      .join("\n");

    let number_color = egui::Color32::from_gray(150);
    let number_font = egui::FontId::monospace(12.0);
    let numbers = egui::RichText::new(line_numbers)
      .font(number_font.clone())
      .color(number_color);

    ui.horizontal(|ui| {
      ui.spacing_mut().item_spacing = egui::vec2(12.0, 0.0);
      ui.add(egui::Label::new(numbers).selectable(false));

      let theme = egui_extras::syntax_highlighting::CodeTheme::dark();
      let mut job = egui_extras::syntax_highlighting::highlight(
        ui.ctx(),
        &theme,
        "rs",
        code,
      );
      job.wrap.max_width = f32::INFINITY;
      ui.add(egui::Label::new(job).selectable(false));
    });
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
        if width_changed {
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
          .rounding(egui::Rounding::same(3.0))
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
                ui.label("+ X");
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
                error_frame.show(ui, |ui| {
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
                    ui.label(
                      egui::RichText::new(err)
                        .color(egui::Color32::from_rgb(255, 220, 220)),
                    );
                  });
                });
                ui.add_space(10.0);
                return;
              }

              if let Some(response) = &self.response {
                let response_text = response.text.clone();
                let response_code = response.code.clone();
                let text_color = self.text_color();
                let divider_color = self.border_color();
                let left_width = ui.available_width() * 0.48;
                ui.horizontal(|ui| {
                  ui.allocate_ui_with_layout(
                    egui::vec2(left_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                      ui.visuals_mut().override_text_color = Some(text_color);
                      CommonMarkViewer::new("response_markdown")
                        .show(ui, &mut self.markdown_cache, &response_text);
                    },
                  );
                  draw_vertical_divider(ui, ui.available_height(), divider_color, 2.0);
                  ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                      let code = if Self::FORCE_SAMPLE_CODE {
                        Self::SAMPLE_CODE
                      } else {
                        let code = response_code.trim();
                        let placeholder = code.is_empty()
                          || code.eq_ignore_ascii_case("rs")
                          || code.eq_ignore_ascii_case("rust");
                        if placeholder { Self::SAMPLE_CODE } else { code }
                      };
                      Self::code_with_line_numbers(ui, code);
                    },
                  );
                });
              }
            });
            let frame_rect = response.response.rect;
            self.show_response_status_overlay(ctx, frame_rect);
            content_size = Some(frame_rect.size());
          });

        if let Some(size) = content_size {
          let desired_height = size
            .y
            .clamp(120.0, Self::RESPONSE_MAX_HEIGHT)
            .ceil();
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
