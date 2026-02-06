use std::fs;
use std::io::BufRead;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use reqwest::header::AUTHORIZATION;
use std::error::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse {
  pub text: String,
  pub code: String,
}

#[derive(Deserialize)]
struct ErrorBody {
  error: ErrorDetail,
}

#[derive(Deserialize)]
struct ErrorDetail {
  code: i32,
  message: String,
}

pub enum WorkerResult {
  Uploading(u64),
  StreamDelta(u64, String),
  Ok(u64, ApiResponse),
  Err(u64, String),
}

pub fn capture_and_upload(
  api_url: &str,
  tx: &mpsc::Sender<WorkerResult>,
  screen_point: Option<(i32, i32)>,
  auth_token: Option<String>,
  model: Option<String>,
  request_id: u64,
) {
  match capture_and_upload_inner(
    api_url,
    tx,
    screen_point,
    auth_token.as_deref(),
    model.as_deref(),
    request_id,
  ) {
    Ok(response) => {
      let _ = tx.send(WorkerResult::Ok(request_id, response));
    }
    Err(err) => {
      let _ = tx.send(WorkerResult::Err(request_id, err));
    }
  }
}

fn capture_and_upload_inner(
  api_url: &str,
  tx: &mpsc::Sender<WorkerResult>,
  screen_point: Option<(i32, i32)>,
  auth_token: Option<&str>,
  model: Option<&str>,
  request_id: u64,
) -> Result<ApiResponse, String> {
  let screen = if let Some((x, y)) = screen_point {
    if let Ok(screen) = screenshots::Screen::from_point(x, y) {
      screen
    } else {
      let screens = screenshots::Screen::all().map_err(|e| e.to_string())?;
      screens
        .into_iter()
        .next()
        .ok_or_else(|| "No screens found".to_string())?
    }
  } else {
    let screens = screenshots::Screen::all().map_err(|e| e.to_string())?;
    screens
      .into_iter()
      .next()
      .ok_or_else(|| "No screens found".to_string())?
  };
  let image = screen.capture().map_err(|e| e.to_string())?;
  let _ = tx.send(WorkerResult::Uploading(request_id));

  let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_err(|e| e.to_string())?
    .as_millis();
  let temp_path = std::env::temp_dir().join(format!("faux_capture_{timestamp}.png"));
  image.save(&temp_path).map_err(|e| e.to_string())?;

  let bytes = fs::read(&temp_path).map_err(|e| e.to_string())?;
  let _ = fs::remove_file(&temp_path);

  let byte_len = bytes.len();
  let part = reqwest::blocking::multipart::Part::bytes(bytes)
    .file_name("screenshot.png")
    .mime_str("image/png")
    .map_err(|e| e.to_string())?;
  let form = reqwest::blocking::multipart::Form::new().part("file", part);

  let timeout_secs = std::env::var("API_TIMEOUT_SECS")
    .ok()
    .and_then(|val| val.parse::<u64>().ok())
    .unwrap_or(180);
  let client = reqwest::blocking::Client::builder()
    .timeout(Duration::from_secs(timeout_secs))
    .connect_timeout(Duration::from_secs(10))
    .build()
    .map_err(|e| e.to_string())?;
  let mut request = client.post(api_url).multipart(form);
  if let Some(token) = auth_token {
    let token = token.trim();
    if !token.is_empty() {
      request = request.bearer_auth(token);
    } else if cfg!(debug_assertions) {
      eprintln!("Auth token present but empty after trim.");
    }
  }
  if let Some(model) = model {
    let model = model.trim();
    if !model.is_empty() {
      request = request.header("x-model", model);
    }
  }
  let request = request.build().map_err(|e| map_request_error(api_url, e))?;
  if cfg!(debug_assertions) {
    log_request_details(&request, byte_len);
  }
  let response = client
    .execute(request)
    .map_err(|e| map_request_error(api_url, e))?;
  let status = response.status();
  if cfg!(debug_assertions) {
    log_response_details(status, response.headers());
  }
  let is_stream = response
    .headers()
    .get(reqwest::header::CONTENT_TYPE)
    .and_then(|val| val.to_str().ok())
    .map(|ct| ct.contains("text/event-stream"))
    .unwrap_or(false);

  if is_stream {
    return read_streaming_response(response, tx, request_id);
  }

  let body_bytes = response
    .bytes()
    .map_err(|e| map_request_error(api_url, e))?;
  let body_text = String::from_utf8_lossy(&body_bytes).to_string();

  if !status.is_success() {
    if let Ok(parsed) = serde_json::from_slice::<ErrorBody>(&body_bytes) {
      let code = parsed.error.code;
      let message = parsed.error.message;
      if !message.is_empty() {
        return Err(format!("Error ({code}): {message}"));
      }
    }
    if cfg!(debug_assertions) {
      eprintln!("API error: {status}: {body_text}");
    }
    if body_text.is_empty() {
      return Err(format!("API returned {status}."));
    }
    return Err(format!("API returned {status}: {body_text}"));
  }

  serde_json::from_slice::<ApiResponse>(&body_bytes).map_err(|err| {
    if cfg!(debug_assertions) {
      eprintln!("API response parse error: {err}. Body: {body_text}");
    }
    "Server returned an invalid response. Please try again or check server logs.".to_string()
  })
}

#[derive(Deserialize)]
struct StreamEnvelope {
  #[serde(rename = "type")]
  kind: String,
  data: Option<String>,
  error: Option<ErrorDetail>,
}

fn read_streaming_response(
  response: reqwest::blocking::Response,
  tx: &mpsc::Sender<WorkerResult>,
  request_id: u64,
) -> Result<ApiResponse, String> {
  let mut reader = std::io::BufReader::new(response);
  let mut full_text = String::new();

  loop {
    let mut line = String::new();
    let bytes = reader
      .read_line(&mut line)
      .map_err(|e| format!("Failed to read stream: {e}"))?;
    if bytes == 0 {
      break;
    }
    let line = line.trim_end();
    if line.is_empty() {
      continue;
    }
    let Some(data) = line.strip_prefix("data:") else {
      continue;
    };
    let payload = data.trim();
    if payload.is_empty() || payload == "[DONE]" {
      continue;
    }
    if let Ok(event) = serde_json::from_str::<StreamEnvelope>(payload) {
      match event.kind.as_str() {
        "delta" => {
          if let Some(delta) = event.data {
            if !delta.is_empty() {
              full_text.push_str(&delta);
              let _ = tx.send(WorkerResult::StreamDelta(request_id, delta));
            }
          }
        }
        "done" => {
          if let Some(text) = event.data {
            if !text.is_empty() {
              full_text = text;
            }
          }
          return Ok(ApiResponse {
            text: full_text,
            code: String::new(),
          });
        }
        "error" => {
          if let Some(err) = event.error {
            return Err(format!("Error ({}): {}", err.code, err.message));
          }
          return Err("Server returned an error.".to_string());
        }
        _ => {}
      }
    }
  }

  Ok(ApiResponse {
    text: full_text,
    code: String::new(),
  })
}

fn map_request_error(api_url: &str, err: reqwest::Error) -> String {
  if cfg!(debug_assertions) {
    eprintln!("Network error for {api_url}: {err}");
    eprintln!(
      "Error flags: connect={} timeout={} body={} decode={}",
      err.is_connect(),
      err.is_timeout(),
      err.is_body(),
      err.is_decode()
    );
    if let Some(status) = err.status() {
      eprintln!("Error status: {status}");
    }
    let mut source = Error::source(&err);
    while let Some(src) = source {
      eprintln!("  caused by: {src}");
      source = src.source();
    }
  }
  if err.is_connect() {
    return format!(
      "Could not connect to the server at {api_url}. Is it running?"
    );
  }
  if err.is_timeout() {
    let timeout_secs = std::env::var("API_TIMEOUT_SECS")
      .ok()
      .and_then(|val| val.parse::<u64>().ok())
      .unwrap_or(180);
    return format!(
      "The request timed out after {timeout_secs}s. Please try again or use a faster model."
    );
  }
  if err.is_body() || err.is_decode() {
    return "Server response could not be read. Please try again.".to_string();
  }
  if cfg!(debug_assertions) {
    return format!("Request failed: {err}");
  }
  "Request failed. Please try again.".to_string()
}

fn log_request_details(request: &reqwest::blocking::Request, byte_len: usize) {
  eprintln!("Request: {} {}", request.method(), request.url());
  eprintln!("Request headers:");
  let mut token_len: Option<usize> = None;
  for (name, value) in request.headers() {
    if name == AUTHORIZATION {
      let value_str = value.to_str().unwrap_or("");
      let parts: Vec<&str> = value_str.splitn(2, ' ').collect();
      if parts.len() == 2 {
        token_len = Some(parts[1].len());
      }
      eprintln!("  {name}: Bearer ***");
    } else {
      let value = value.to_str().unwrap_or("<binary>");
      eprintln!("  {name}: {value}");
    }
  }
  if let Some(len) = token_len {
    eprintln!("Auth token length: {len}");
  } else {
    eprintln!("Auth token not present");
  }
  eprintln!("Upload bytes: {byte_len}");
}

fn log_response_details(status: reqwest::StatusCode, headers: &reqwest::header::HeaderMap) {
  eprintln!("Response status: {}", status);
  eprintln!("Response headers:");
  for (name, value) in headers {
    let value = value.to_str().unwrap_or("<binary>");
    eprintln!("  {name}: {value}");
  }
}
