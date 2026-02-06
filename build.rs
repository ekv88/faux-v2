use std::path::Path;

fn main() {
  #[cfg(target_os = "windows")]
  {
    let icon_path = Path::new("assets").join("icon.ico");
    if icon_path.exists() {
      let mut res = winres::WindowsResource::new();
      if let Some(icon_str) = icon_path.to_str() {
        res.set_icon(icon_str);
        let _ = res.compile();
      }
    }
  }
}
