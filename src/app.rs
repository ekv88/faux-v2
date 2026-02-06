use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicU32, Ordering},
  mpsc,
};
use std::str::FromStr;

use eframe::egui;
use egui_commonmark::CommonMarkCache;
use egui_phosphor as phosphor;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::api::{ApiResponse, WorkerResult, capture_and_upload};
use crate::config::{AppConfig, WindowPosition, current_dir_config_path, read_config, write_config};
use crate::ui::{draw_vertical_divider, install_phosphor_fonts};

mod response_window;
mod settings_window;

pub fn run() -> eframe::Result<()> {
  dotenvy::dotenv().ok();
  let config = read_config(&current_dir_config_path());

    let mut viewport = egui::ViewportBuilder::default()
      .with_title("Faux")
      .with_inner_size([320.0, 24.0])
      .with_resizable(false)
      .with_decorations(false)
      .with_transparent(true)
      .with_taskbar(false);
    if config.always_on_top {
      viewport = viewport.with_always_on_top();
    }
  if let Some(pos) = config.main_position {
    viewport = viewport.with_position([pos.x, pos.y]);
  }

  let options = eframe::NativeOptions {
    renderer: eframe::Renderer::Glow,
    viewport,
    ..Default::default()
  };

  eframe::run_native("Faux", options, Box::new(|cc| Box::new(AppState::new(cc))))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HotKeys {
  show_hide: HotKey,
  screenshot: HotKey,
  close_response: HotKey,
  quit: HotKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HotkeyAction {
  ShowHide,
  Screenshot,
  CloseResponse,
  Quit,
}

struct AppState {
  config: AppConfig,
  config_path: std::path::PathBuf,
  api_url: String,

  hotkeys: HotKeys,
  _hotkey_manager: GlobalHotKeyManager,
  hotkey_rx: mpsc::Receiver<GlobalHotKeyEvent>,
  show_hide_id: Arc<AtomicU32>,

  worker_tx: mpsc::Sender<WorkerResult>,
  worker_rx: mpsc::Receiver<WorkerResult>,

  main_visible: bool,
  main_visible_atomic: Arc<AtomicBool>,
  main_size: egui::Vec2,
  response_open: bool,
  settings_open: bool,
  confirm_quit_open: bool,
  loading: bool,
  response: Option<ApiResponse>,
  last_error: Option<String>,
  response_status: Option<String>,
  response_size: egui::Vec2,
  response_hwnd_hooked: bool,
  response_last_pos: Option<egui::Pos2>,
  response_scroll_offset: f32,
  response_scroll_max: f32,
  main_fade: f32,
  main_dragging: bool,
  hotkey_capture: Option<HotkeyAction>,
  config_dirty: bool,
  last_config_save: std::time::Instant,
  main_hwnd: Option<isize>,
    main_hwnd_hooked: bool,
    settings_hwnd_hooked: bool,
    last_screen_point: Option<(i32, i32)>,
    last_saved_pos: Option<egui::Pos2>,
    last_position_write: std::time::Instant,
    quit_requested: bool,
    markdown_cache: CommonMarkCache,
    background_picker_open: bool,
    text_picker_open: bool,
    divider_picker_open: bool,
  }

  impl AppState {
    const MAIN_LETTER_SPACING: f32 = -0.5;
  const RESPONSE_MIN_WIDTH: f32 = 470.0;
  const RESPONSE_MAX_WIDTH: f32 = 860.0;
  const RESPONSE_HEIGHT: f32 = 400.0;
  const RESPONSE_ANCHOR_GAP: f32 = 10.0;
    const RESPONSE_TITLE: &'static str = "Faux Response";

  fn parse_hotkey_spec(spec: &str, fallback: &str) -> HotKey {
    HotKey::from_str(spec)
      .or_else(|_| HotKey::from_str(fallback))
      .unwrap_or_else(|_| HotKey::new(Some(Modifiers::CONTROL), Code::KeyH))
  }

  fn normalize_hotkey_token(token: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
      return None;
    }
    let upper = trimmed.to_uppercase();
    if upper == "ESC" || upper == "ESCAPE" {
      return Some("Escape".to_string());
    }
    if upper.starts_with('F')
      && upper.len() <= 3
      && upper[1..].chars().all(|c| c.is_ascii_digit())
    {
      return Some(upper);
    }
    if trimmed.len() == 1 {
      let ch = trimmed.chars().next().unwrap();
      if ch.is_ascii_alphabetic() {
        return Some(format!("Key{}", ch.to_ascii_uppercase()));
      }
      if ch.is_ascii_digit() {
        return Some(format!("Digit{}", ch));
      }
    }
    None
  }

  fn hotkey_spec_from_token(token: &str, action: HotkeyAction) -> Option<String> {
    let key = Self::normalize_hotkey_token(token)?;
    let spec = match action {
      HotkeyAction::Quit => format!("CmdOrCtrl+{key}"),
      _ => format!("CmdOrCtrl+{key}"),
    };
    Some(spec)
  }

  fn hotkeys_from_config(config: &AppConfig) -> HotKeys {
    let show_hide_spec = Self::hotkey_spec_from_token(&config.hotkeys.show_hide, HotkeyAction::ShowHide)
      .unwrap_or_else(|| "CmdOrCtrl+KeyH".to_string());
    let screenshot_spec = Self::hotkey_spec_from_token(&config.hotkeys.screenshot, HotkeyAction::Screenshot)
      .unwrap_or_else(|| "CmdOrCtrl+KeyQ".to_string());
    let close_spec = Self::hotkey_spec_from_token(&config.hotkeys.close_response, HotkeyAction::CloseResponse)
      .unwrap_or_else(|| "CmdOrCtrl+KeyX".to_string());
    let quit_spec = Self::hotkey_spec_from_token(&config.hotkeys.quit, HotkeyAction::Quit)
      .unwrap_or_else(|| "CmdOrCtrl+Escape".to_string());

    let show_hide = Self::parse_hotkey_spec(&show_hide_spec, "CmdOrCtrl+KeyH");
    let screenshot = Self::parse_hotkey_spec(&screenshot_spec, "CmdOrCtrl+KeyQ");
    let close_response = Self::parse_hotkey_spec(&close_spec, "CmdOrCtrl+KeyX");
    let quit = Self::parse_hotkey_spec(&quit_spec, "CmdOrCtrl+Escape");
    HotKeys {
      show_hide,
      screenshot,
      close_response,
      quit,
    }
  }

  fn register_hotkeys_with_fallback(
    manager: &GlobalHotKeyManager,
    mut hotkeys: HotKeys,
  ) -> Result<(HotKeys, Option<String>), String> {
    manager
      .register(hotkeys.show_hide)
      .map_err(|e| format!("show/hide hotkey: {e}"))?;
    manager
      .register(hotkeys.screenshot)
      .map_err(|e| format!("screenshot hotkey: {e}"))?;
    manager
      .register(hotkeys.close_response)
      .map_err(|e| format!("close-response hotkey: {e}"))?;

    if manager.register(hotkeys.quit).is_err() {
      let fallback = Self::parse_hotkey_spec("CmdOrCtrl+KeyP", "CmdOrCtrl+KeyP");
      if manager.register(fallback).is_ok() {
        hotkeys.quit = fallback;
        return Ok((hotkeys, Some("P".to_string())));
      }
      let fallback_alt = Self::parse_hotkey_spec("CmdOrCtrl+KeyO", "CmdOrCtrl+KeyO");
      if manager.register(fallback_alt).is_ok() {
        hotkeys.quit = fallback_alt;
        return Ok((hotkeys, Some("O".to_string())));
      }
      return Ok((hotkeys, None));
    }

    Ok((hotkeys, None))
  }

  fn apply_hotkeys_from_config(&mut self) {
    let desired_hotkeys = Self::hotkeys_from_config(&self.config);
    if desired_hotkeys == self.hotkeys {
      return;
    }

    let old = self.hotkeys;
    let _ = self._hotkey_manager.unregister(old.show_hide);
    let _ = self._hotkey_manager.unregister(old.screenshot);
    let _ = self._hotkey_manager.unregister(old.close_response);
    let _ = self._hotkey_manager.unregister(old.quit);

    let (registered, quit_token) =
      match Self::register_hotkeys_with_fallback(&self._hotkey_manager, desired_hotkeys) {
        Ok(result) => result,
        Err(err) => {
          eprintln!("Failed to register hotkeys: {err}");
          let _ = Self::register_hotkeys_with_fallback(&self._hotkey_manager, old);
          return;
        }
      };

    if let Some(token) = quit_token {
      self.config.hotkeys.quit = token;
      self.save_config();
    }

    self.hotkeys = registered;
    self.show_hide_id
      .store(self.hotkeys.show_hide.id(), Ordering::SeqCst);
  }

  fn try_register_hotkeys_on_start(
    config: &mut AppConfig,
    manager: &GlobalHotKeyManager,
  ) -> HotKeys {
    let desired = Self::hotkeys_from_config(config);
    let (registered, quit_token) =
      Self::register_hotkeys_with_fallback(manager, desired)
        .expect("failed to register hotkeys");
    if let Some(token) = quit_token {
      config.hotkeys.quit = token;
    }
    registered
  }

  fn hotkey_label_from_token(token: &str) -> String {
    let trimmed = token.trim();
    if trimmed.is_empty() {
      return "?".to_string();
    }
    let upper = trimmed.to_uppercase();
    if upper == "ESC" || upper == "ESCAPE" {
      return "Esc".to_string();
    }
    if upper.starts_with('F')
      && upper.len() <= 3
      && upper[1..].chars().all(|c| c.is_ascii_digit())
    {
      return upper;
    }
    if trimmed.len() == 1 {
      return trimmed.to_ascii_uppercase();
    }
    trimmed.to_string()
  }

  fn update_hotkey_binding(&mut self, action: HotkeyAction, token: String) {
    eprintln!("Update hotkey {:?} -> {}", action, token);
    match action {
      HotkeyAction::ShowHide => self.config.hotkeys.show_hide = token,
      HotkeyAction::Screenshot => self.config.hotkeys.screenshot = token,
      HotkeyAction::CloseResponse => self.config.hotkeys.close_response = token,
      HotkeyAction::Quit => self.config.hotkeys.quit = token,
    }
    self.apply_hotkeys_from_config();
    self.save_config();
  }

  fn egui_key_to_token(key: egui::Key) -> Option<String> {
    if key == egui::Key::Escape {
      return Some("ESC".to_string());
    }
    let name = key.name();
    if name.len() == 1 && name.chars().all(|c| c.is_ascii_alphanumeric()) {
      return Some(name.to_ascii_uppercase());
    }
    if name.starts_with('F') && name[1..].chars().all(|c| c.is_ascii_digit()) {
      return Some(name.to_string());
    }
    None
  }

  fn text_color(&self) -> egui::Color32 {
    self.config.text_color.to_color32()
  }

  fn fade_color(color: egui::Color32, factor: f32) -> egui::Color32 {
    let factor = factor.clamp(0.0, 1.0);
    let alpha = (color.a() as f32 * factor).round().clamp(0.0, 255.0) as u8;
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
  }

  fn background_color_main(&self) -> egui::Color32 {
    let base = self.config.background.to_color32();
    let opacity = (self.config.opacity * self.main_fade).clamp(0.0, 1.0);
    Self::apply_opacity(base, opacity)
  }

  fn border_color_main(&self) -> egui::Color32 {
    Self::darken(self.background_color_main(), 0.6)
  }

    fn background_color(&self) -> egui::Color32 {
      let base = self.config.background.to_color32();
      Self::apply_opacity(base, self.config.opacity)
    }

    fn background_color_layered(&self) -> egui::Color32 {
      let base = self.config.background.to_color32();
      egui::Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), base.a())
    }

    fn color_swatch(ui: &mut egui::Ui, color: egui::Color32) -> egui::Response {
      let size = egui::vec2(28.0, 16.0);
      let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
      let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(80));
      ui.painter().rect(rect, egui::Rounding::same(3.0), color, stroke);
      response
    }

    fn border_color(&self) -> egui::Color32 {
      Self::darken(self.background_color(), 0.6)
    }

    fn divider_color(&self) -> egui::Color32 {
      self.config.divider_color.to_color32()
    }

    fn button_fill(&self, hovered: bool) -> egui::Color32 {
      let base = self.background_color();
      let factor = if hovered { 0.95 } else { 0.8 };
      Self::apply_opacity(base, factor)
    }

    fn button_border(&self) -> egui::Color32 {
      Self::darken(self.background_color(), 0.7)
    }

    fn skeleton_color(&self) -> egui::Color32 {
      let color = self.text_color();
      egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 24)
    }

    fn apply_opacity(color: egui::Color32, opacity: f32) -> egui::Color32 {
      let alpha = ((color.a() as f32) * opacity.clamp(0.0, 1.0)).round() as u8;
      egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
    }

    fn darken(color: egui::Color32, factor: f32) -> egui::Color32 {
      let f = factor.clamp(0.0, 1.0);
      egui::Color32::from_rgba_unmultiplied(
        (color.r() as f32 * f) as u8,
        (color.g() as f32 * f) as u8,
        (color.b() as f32 * f) as u8,
        color.a(),
      )
    }

  #[cfg(target_os = "windows")]
  fn apply_windows_tool_window(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
      GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    };
    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
    let mut new_style = ex_style | WS_EX_TOOLWINDOW.0 as i32;
    new_style &= !(WS_EX_APPWINDOW.0 as i32);
    if new_style != ex_style {
      unsafe {
        let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
      }
    }
  }

  #[cfg(target_os = "windows")]
  fn find_window_by_title(title: &str) -> Option<windows::Win32::Foundation::HWND> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    let title: Vec<u16> = title
      .encode_utf16()
      .chain(std::iter::once(0))
      .collect();
    let hwnd = unsafe { FindWindowW(None, PCWSTR::from_raw(title.as_ptr())) };
    if hwnd.0 == 0 { None } else { Some(hwnd) }
  }

  #[cfg(target_os = "windows")]
  fn apply_windows_exclude_from_capture(hwnd: windows::Win32::Foundation::HWND, enabled: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{
      SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    };
    unsafe {
      let _ = SetWindowDisplayAffinity(
        hwnd,
        if enabled { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE },
      );
    }
  }

  fn new(cc: &eframe::CreationContext<'_>) -> Self {
    let config_path = current_dir_config_path();
    let mut config = read_config(&config_path);
    let api_url =
      std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:3005/ingest".to_string());

    let hotkey_manager = GlobalHotKeyManager::new().expect("global hotkeys must be available");
    let hotkeys = Self::try_register_hotkeys_on_start(&mut config, &hotkey_manager);

    let (worker_tx, worker_rx) = mpsc::channel();
    let (hotkey_tx, hotkey_rx) = mpsc::channel();

    let main_visible_atomic = Arc::new(AtomicBool::new(true));
    let show_hide_id = Arc::new(AtomicU32::new(hotkeys.show_hide.id()));
    let repaint_ctx = cc.egui_ctx.clone();
    let visible_flag = Arc::clone(&main_visible_atomic);
    let show_hide_id_atomic = Arc::clone(&show_hide_id);
    #[cfg(target_os = "windows")]
    let response_title = Self::RESPONSE_TITLE.to_string();

    let main_hwnd = {
      #[cfg(target_os = "windows")]
      {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        cc.window_handle()
          .ok()
          .and_then(|handle| match handle.as_raw() {
            RawWindowHandle::Win32(win) => Some(win.hwnd.get()),
            _ => None,
          })
      }
      #[cfg(not(target_os = "windows"))]
      {
        None
      }
    };
    #[cfg(target_os = "windows")]
    let main_hwnd_for_thread = main_hwnd;

    std::thread::spawn(move || {
      let hotkey_events = GlobalHotKeyEvent::receiver();
      while let Ok(event) = hotkey_events.recv() {
        if event.id == show_hide_id_atomic.load(Ordering::SeqCst) {
          if event.state != HotKeyState::Pressed {
            continue;
          }
          let new_visible = !visible_flag.load(Ordering::SeqCst);
          visible_flag.store(new_visible, Ordering::SeqCst);

          #[cfg(target_os = "windows")]
          {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{
              FindWindowW, SW_HIDE, SW_SHOW, ShowWindow,
            };
            use windows::core::PCWSTR;

            if let Some(hwnd) = main_hwnd_for_thread {
              unsafe {
                ShowWindow(HWND(hwnd), if new_visible { SW_SHOW } else { SW_HIDE });
              }
            }

            let title: Vec<u16> = response_title
              .encode_utf16()
              .chain(std::iter::once(0))
              .collect();
            let response_hwnd = unsafe { FindWindowW(None, PCWSTR::from_raw(title.as_ptr())) };
            if response_hwnd.0 != 0 {
              unsafe {
                ShowWindow(response_hwnd, if new_visible { SW_SHOW } else { SW_HIDE });
              }
            }
          }

          repaint_ctx.request_repaint();
          continue;
        }

        let _ = hotkey_tx.send(event);
        repaint_ctx.request_repaint();
      }
    });

    cc.egui_ctx.set_visuals(egui::Visuals::dark());
    install_phosphor_fonts(&cc.egui_ctx);

    if write_config(&config_path, &config).is_ok() {
      // Persist any fallback hotkey adjustments.
    }

    Self {
      config: config.clone(),
      config_path,
      api_url,
      hotkeys,
      _hotkey_manager: hotkey_manager,
      hotkey_rx,
      show_hide_id,
      worker_tx,
      worker_rx,
      main_visible: true,
      main_visible_atomic,
      main_size: egui::vec2(320.0, 24.0),
      response_open: false,
      settings_open: false,
      confirm_quit_open: false,
      loading: false,
      response: None,
      last_error: None,
      response_status: None,
      response_size: egui::vec2(Self::RESPONSE_MAX_WIDTH, Self::RESPONSE_HEIGHT),
      response_hwnd_hooked: false,
      response_last_pos: None,
      response_scroll_offset: 0.0,
      response_scroll_max: 0.0,
      main_fade: 0.0,
      main_dragging: false,
      hotkey_capture: None,
      config_dirty: false,
      last_config_save: std::time::Instant::now(),
      main_hwnd,
      main_hwnd_hooked: false,
      settings_hwnd_hooked: false,
      last_screen_point: None,
        last_saved_pos: config.main_position.map(|pos| egui::pos2(pos.x, pos.y)),
        last_position_write: std::time::Instant::now(),
        quit_requested: false,
        markdown_cache: CommonMarkCache::default(),
        background_picker_open: false,
        text_picker_open: false,
        divider_picker_open: false,
      }
  }

  fn process_hotkeys(&mut self, ctx: &egui::Context) {
    if self.hotkey_capture.is_some() {
      while self.hotkey_rx.try_recv().is_ok() {}
      return;
    }
    while let Ok(event) = self.hotkey_rx.try_recv() {
      if event.id == self.hotkeys.show_hide.id() {
        if event.state != HotKeyState::Pressed {
          continue;
        }
        self.main_visible = !self.main_visible;
        self
          .main_visible_atomic
          .store(self.main_visible, Ordering::SeqCst);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.main_visible));
        if self.main_visible {
          ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
      } else if event.id == self.hotkeys.screenshot.id() {
        self.start_capture(ctx);
      } else if event.id == self.hotkeys.close_response.id() {
        self.close_response();
      } else if event.id == self.hotkeys.quit.id() {
        if event.state == HotKeyState::Pressed {
          self.quit_requested = true;
        }
      }
    }
  }

  fn process_hotkey_capture(&mut self, ctx: &egui::Context) {
    let Some(action) = self.hotkey_capture else {
      return;
    };

    let mut captured: Option<String> = None;
    ctx.input(|i| {
      for event in &i.events {
        if let egui::Event::Key { key, pressed: true, .. } = event {
          if let Some(token) = Self::egui_key_to_token(*key) {
            captured = Some(token);
            break;
          }
        }
      }
    });

    if let Some(token) = captured {
      self.update_hotkey_binding(action, token);
      self.hotkey_capture = None;
    }
  }

  fn process_worker_results(&mut self) {
    while let Ok(result) = self.worker_rx.try_recv() {
      match result {
        WorkerResult::Uploading => {
          self.response_status = Some("Uploading...".to_string());
          self.loading = true;
        }
          WorkerResult::Ok(mut response) => {
            let trimmed = response.code.trim();
            if trimmed.eq_ignore_ascii_case("rs")
              || trimmed.eq_ignore_ascii_case("rust")
              || trimmed.eq_ignore_ascii_case("code")
            {
              response.code.clear();
            }
            self.loading = false;
            self.response = Some(response);
            self.last_error = None;
            self.response_status = Some("Ready".to_string());
          }
        WorkerResult::Err(err) => {
          self.loading = false;
          self.response = None;
          self.last_error = Some(err);
          self.response_status = Some("Error".to_string());
        }
      }
    }
  }

  fn start_capture(&mut self, ctx: &egui::Context) {
    if self.loading {
      return;
    }
    self.update_last_screen_point(ctx);
    self.loading = true;
    self.response_open = true;
    self.response_hwnd_hooked = false;
    self.response_last_pos = None;
    self.response = None;
    self.last_error = None;
    self.response_status = Some("Capturing...".to_string());
    self.response_scroll_offset = 0.0;
    self.response_scroll_max = 0.0;
    self.response_scroll_offset = 0.0;

    let api_url = self.api_url.clone();
    let auth_token = self
      .config
      .api_key
      .trim()
      .to_string();
    let tx = self.worker_tx.clone();
    let capture_point = self.last_screen_point;
    std::thread::spawn(move || {
      let token = if auth_token.is_empty() { None } else { Some(auth_token) };
      capture_and_upload(&api_url, &tx, capture_point, token);
    });
  }

  fn close_response(&mut self) {
    self.response_open = false;
    self.loading = false;
    self.response = None;
    self.last_error = None;
    self.response_hwnd_hooked = false;
    self.response_last_pos = None;
    self.response_status = None;
    self.response_scroll_offset = 0.0;
    self.response_scroll_max = 0.0;
  }

  fn save_config(&self) {
    let _ = write_config(&self.config_path, &self.config);
  }

  fn schedule_config_save(&mut self) {
    self.config_dirty = true;
  }

  fn flush_config_if_needed(&mut self) {
    if !self.config_dirty {
      return;
    }
    if self.last_config_save.elapsed().as_millis() < 150 {
      return;
    }
    self.save_config();
    self.config_dirty = false;
    self.last_config_save = std::time::Instant::now();
  }

  fn maybe_save_position(&mut self, ctx: &egui::Context) {
    let Some(outer) = ctx.input(|i| i.viewport().outer_rect) else {
      return;
    };
    let pos = outer.min;
    let should_write = match self.last_saved_pos {
      Some(prev) => (pos - prev).length_sq() > 1.0,
      None => true,
    };
    if !should_write {
      return;
    }
    if self.last_position_write.elapsed().as_millis() < 250 {
      return;
    }

    self.last_saved_pos = Some(pos);
    self.last_position_write = std::time::Instant::now();
    self.config.main_position = Some(WindowPosition { x: pos.x, y: pos.y });
    self.save_config();
  }

  fn update_main_size(&mut self, ctx: &egui::Context, desired: egui::Vec2) {
    let desired = egui::vec2(desired.x.ceil(), desired.y.ceil());
    if (desired - self.main_size).length_sq() > 0.5 {
      self.main_size = desired;
      ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(desired));
    }
  }

  fn sync_visibility(&mut self, ctx: &egui::Context) {
    let desired = self.main_visible_atomic.load(Ordering::SeqCst);
    if desired != self.main_visible {
      self.main_visible = desired;
      if !self.main_visible {
        self.settings_open = false;
        self.hotkey_capture = None;
      }
      if self.main_visible {
        self.main_fade = 0.0;
      }
      ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.main_visible));
      if self.main_visible {
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
      }
    }
  }

  fn update_last_screen_point(&mut self, ctx: &egui::Context) {
    let Some(outer) = ctx.input(|i| i.viewport().outer_rect) else {
      return;
    };
    let center = outer.center();
    let scale = ctx.pixels_per_point();
    let x = (center.x * scale).round() as i32;
    let y = (center.y * scale).round() as i32;
    self.last_screen_point = Some((x, y));
  }


  fn main_text(&self, text: impl Into<String>) -> egui::WidgetText {
    egui::WidgetText::from(
      egui::RichText::new(text.into())
        .strong()
        .extra_letter_spacing(Self::MAIN_LETTER_SPACING),
    )
  }

  fn main_icon(&self, icon: &str, size: f32) -> egui::WidgetText {
    egui::WidgetText::from(
      egui::RichText::new(icon)
        .size(size)
        .strong()
        .extra_letter_spacing(Self::MAIN_LETTER_SPACING),
    )
  }

  fn main_label(ui: &mut egui::Ui, text: egui::WidgetText) {
    ui.add(egui::Label::new(text).selectable(false));
  }

  fn icon_badge(
    &self,
    ui: &mut egui::Ui,
    icon: &str,
    size: f32,
    padding: f32,
    y_offset: f32,
    clickable: bool,
    border: bool,
  ) -> egui::Response {
    let total = size + padding * 2.0;
    let sense = if clickable {
      egui::Sense::click()
    } else {
      egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(total, total), sense);
    self.paint_badge(ui, rect, response.hovered(), border);

    let font_id = egui::FontId::new(size, egui::FontFamily::Proportional);
    let color = self.text_color();
    let pos = rect.center() + egui::vec2(0.0, y_offset);
    ui.painter()
      .text(pos, egui::Align2::CENTER_CENTER, icon, font_id, color);
    response
  }

  fn text_badge(
    &self,
    ui: &mut egui::Ui,
    text: &str,
    padding_x: f32,
    padding_y: f32,
    clickable: bool,
  ) -> egui::Response {
    let text = self.main_text(text);
    let galley = text.into_galley(ui, Some(false), f32::INFINITY, egui::TextStyle::Body);
    let total = galley.size() + egui::vec2(padding_x * 2.0, padding_y * 2.0);
    let sense = if clickable {
      egui::Sense::click()
    } else {
      egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(total, sense);
    self.paint_badge(ui, rect, response.hovered(), true);
    let pos = rect.min + (rect.size() - galley.size()) * 0.5;
    ui.painter().galley(pos, galley, ui.visuals().text_color());
    response
  }

  fn paint_badge(&self, ui: &mut egui::Ui, rect: egui::Rect, hovered: bool, border: bool) {
    let rounding = egui::Rounding::same(4.0);
    let fill = self.button_fill(hovered);
    let stroke = if border {
      egui::Stroke::new(1.0, self.button_border())
    } else {
      egui::Stroke::NONE
    };
    ui.painter().rect(rect, rounding, fill, stroke);
  }

  fn modifiers_row(&self, ui: &mut egui::Ui, size: f32) {
    self.icon_badge(ui, phosphor::regular::CONTROL, size, 2.0, 3.0, false, true);
    ui.add_space(-4.0);
    Self::main_label(ui, self.main_text("/"));
    ui.add_space(-3.0);
    self.icon_badge(ui, phosphor::regular::COMMAND, size, 2.0, 0.0, false, true);
    ui.add_space(-3.0);
  }

  fn show_main_window(&mut self, ctx: &egui::Context) {
    #[cfg(target_os = "windows")]
    if !self.main_hwnd_hooked {
      if let Some(hwnd) = self.main_hwnd {
        Self::apply_windows_tool_window(windows::Win32::Foundation::HWND(hwnd));
        self.main_hwnd_hooked = true;
      }
      if !self.main_hwnd_hooked {
        if let Some(hwnd) = Self::find_window_by_title("Faux") {
          Self::apply_windows_tool_window(hwnd);
          self.main_hwnd_hooked = true;
        }
      }
    }
    #[cfg(target_os = "windows")]
    if let Some(hwnd) = self.main_hwnd {
      Self::apply_windows_exclude_from_capture(
        windows::Win32::Foundation::HWND(hwnd),
        self.config.stealth,
      );
    }
    let margin = egui::Margin::symmetric(10.0, 4.0);
    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.config.always_on_top {
      egui::WindowLevel::AlwaysOnTop
    } else {
      egui::WindowLevel::Normal
    }));
    let frame = egui::Frame::none()
      .fill(self.background_color_main())
      .stroke(egui::Stroke::new(1.0, self.border_color_main()))
      .rounding(egui::Rounding::same(5.0))
      .inner_margin(margin);

    egui::CentralPanel::default()
      .frame(egui::Frame::none())
      .show(ctx, |ui| {
        ui.visuals_mut().override_text_color =
          Some(Self::fade_color(self.text_color(), self.main_fade));
        let mut main_row_size = egui::Vec2::ZERO;
        frame.show(ui, |ui| {
          let drag = ui.interact(
            ui.max_rect(),
            ui.id().with("drag"),
            egui::Sense::drag(),
          );
          if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
          }
          self.main_dragging = drag.dragged();
          let row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
            let icon_size = 14.0;
            self.modifiers_row(ui, icon_size);
            Self::main_label(ui, self.main_text("+"));
            ui.add_space(-1.0);
            let show_label = Self::hotkey_label_from_token(&self.config.hotkeys.show_hide);
            self.text_badge(ui, &show_label, 3.0, 2.0, false);
            Self::main_label(ui, self.main_icon(phosphor::regular::EYE_SLASH, icon_size));
            ui.add_space(-1.0);
            Self::main_label(ui, self.main_text("Show/Hide"));
            draw_vertical_divider(ui, 1.5, self.divider_color(), 2.0);
            self.modifiers_row(ui, icon_size);
            Self::main_label(ui, self.main_text("+"));
            ui.add_space(-1.0);
            let shot_label = Self::hotkey_label_from_token(&self.config.hotkeys.screenshot);
            self.text_badge(ui, &shot_label, 3.0, 2.0, false);
            Self::main_label(ui, self.main_icon(phosphor::regular::CAMERA, icon_size));
            Self::main_label(ui, self.main_text("Take screenshot"));

            ui.add_space(1.0);
            draw_vertical_divider(ui, 1.5, self.divider_color(), 2.0);
            let settings_resp = self.icon_badge(
              ui,
              phosphor::regular::GEAR,
              icon_size + 2.0,
              2.0,
              0.0,
              true,
              true,
            )
            .on_hover_text("Settings");
            let clicked = settings_resp.clicked();
            if clicked {
              self.settings_open = !self.settings_open;
              if !self.settings_open {
                self.settings_hwnd_hooked = false;
              }
            }
            if self.confirm_quit_open {
              ui.add_space(-2.0);
                Self::main_label(ui, self.main_text("Quit?"));
              let yes = self.text_badge(ui, "Yes", 3.0, 2.0, true);
              let no = self.text_badge(ui, "No", 3.0, 2.0, true);
              if yes.clicked() {
                self.quit_requested = true;
                self.confirm_quit_open = false;
              }
              if no.clicked() {
                self.confirm_quit_open = false;
              }
            } else {
              let close_resp = self.icon_badge(
                ui,
                phosphor::regular::X,
                icon_size + 2.0,
                2.0,
                0.0,
                true,
                true,
              )
              .on_hover_text("Quit");
              if close_resp.clicked() {
                self.confirm_quit_open = true;
              }
            }
          });
          main_row_size = row.response.rect.size();
        });

        let total_width = main_row_size.x + margin.left + margin.right;
        let total_height = main_row_size.y + margin.top + margin.bottom;

        let desired = egui::vec2(total_width + 2.0, total_height + 2.0);
        if !self.main_dragging {
          self.update_main_size(ctx, desired);
        }
      });
  }

}

impl eframe::App for AppState {
  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    if self.config.theme.eq_ignore_ascii_case("light") {
      ctx.set_visuals(egui::Visuals::light());
    } else {
      ctx.set_visuals(egui::Visuals::dark());
    }
    self.process_hotkeys(ctx);
    self.process_hotkey_capture(ctx);
    self.process_worker_results();
    self.sync_visibility(ctx);
    if self.response_open {
      let delta = ctx.input(|i| i.raw_scroll_delta.y);
      if delta.abs() > 0.0 {
        let next = self.response_scroll_offset - delta;
        let max = self.response_scroll_max.max(0.0);
        self.response_scroll_offset = next.clamp(0.0, max);
      }
    }

    if self.main_visible {
      let dt = ctx.input(|i| i.unstable_dt).clamp(0.0, 0.1);
      if self.main_fade < 1.0 {
        self.main_fade = (self.main_fade + dt * 6.0).min(1.0);
        ctx.request_repaint();
      }
      self.update_last_screen_point(ctx);
      self.show_main_window(ctx);
      self.show_settings_window(ctx);
      self.show_response_window(ctx);
      self.maybe_save_position(ctx);
    }

    if self.quit_requested {
      ctx.send_viewport_cmd(egui::ViewportCommand::Close);
      return;
    }

    ctx.request_repaint_after(std::time::Duration::from_millis(16));
  }

  fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
    egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
  }
}
