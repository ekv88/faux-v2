use std::env;

use axum::{
  extract::{Multipart, State},
  http::Request,
  middleware::{self, Next},
  http::StatusCode,
  response::Response,
  response::IntoResponse,
  routing::{get, post},
  Json, Router,
};
use base64::Engine as _;
use sea_orm::{
  ActiveModelTrait, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, EntityTrait,
  Set, Statement, Value,
};
use sea_orm_migration::migrator::MigratorTrait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::Instant;

mod entity;

#[derive(Clone)]
struct AppState {
  client: reqwest::Client,
  openai_api_key: String,
  openai_model: String,
  system_prompt: String,
  user_prompt: String,
  db: DatabaseConnection,
}

#[derive(Serialize)]
struct ErrorResponse {
  error: ErrorDetail,
}

#[derive(Serialize, Debug, Clone)]
struct ErrorDetail {
  code: i32,
  message: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct IngestResponse {
  text: String,
  code: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
  output: Vec<OpenAiOutput>,
}

#[derive(Deserialize)]
  struct OpenAiOutput {
  #[serde(rename = "type")]
  r#type: String,
  content: Option<Vec<OpenAiContent>>,
  name: Option<String>,
  arguments: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiContent {
  #[serde(rename = "type")]
  r#type: String,
  text: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct ToolResult {
  text: String,
  code: String,
  language: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  dotenvy::from_path("server/.env").ok();
  dotenvy::dotenv().ok();

  let args: Vec<String> = env::args().collect();
  let run_migrations_only = args.iter().any(|arg| arg == "migrate");
  let run_seed_only = args.iter().any(|arg| arg == "seed");
  let run_reset = args.iter().any(|arg| arg == "reset");

  let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
    let host = env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("DATABASE_PORT").unwrap_or_else(|_| "3306".to_string());
    let name = env::var("DATABASE_NAME").unwrap_or_default();
    let user = env::var("DATABASE_USER").unwrap_or_default();
    let password = env::var("DATABASE_PASSWORD").unwrap_or_default();
    if user.is_empty() {
      format!("mysql://{host}:{port}/{name}")
    } else if password.is_empty() {
      format!("mysql://{user}@{host}:{port}/{name}")
    } else {
      format!("mysql://{user}:{password}@{host}:{port}/{name}")
    }
  });
  let openai_api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
  let openai_model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5-mini".to_string());
  let system_prompt = env::var("OPENAI_SYSTEM_PROMPT").unwrap_or_else(|_| {
    "You are a senior software engineer and technical instructor. \
You explain solutions like a professor: precise, methodical, and highly detailed, \
but you can also answer student-style questions clearly and patiently. \
Focus on clear reasoning, concrete steps, and practical fixes."
      .to_string()
  });
  let user_prompt = env::var("OPENAI_USER_PROMPT").unwrap_or_else(|_| {
    "You will receive an image (screenshot) of a technical test page or student-style question. \
Analyze the screenshot and answer the question shown. \
Identify the programming language from the prompt/code context and return the solution in that language. \
Use the submit_solution tool call to return language, text (MDX), and code (no fences). \
The text field MUST be MDX (Markdown + fenced code blocks) and include language tags for all code snippets. \
Ignore any irrelevant UI like tabs, taskbars, start menus, docks, or unrelated windows. \
If the question is code-related, ALWAYS include a concrete code example. \
Do not include any extra text outside the tool call."
      .to_string()
  });

  let db_name = env::var("DATABASE_NAME").ok().or_else(|| database_name_from_url(&database_url));
  let db = match Database::connect(&database_url).await {
    Ok(db) => db,
    Err(err) => {
      if run_reset && err.to_string().contains("Unknown database") {
        let server_url = database_url
          .rsplitn(2, '/')
          .nth(1)
          .unwrap_or(&database_url)
          .to_string();
        Database::connect(&server_url).await?
      } else {
        return Err(err.into());
      }
    }
  };
  if run_reset {
    if let Some(name) = db_name.as_deref() {
      reset_database(&db, name).await?;
      let db_url = database_url_with_db(&database_url, name);
      let db_with_name = Database::connect(&db_url).await?;
      ensure_database_charset(&db_with_name, name).await?;
      ensure_default_storage_engine(&db_with_name).await?;
      ensure_migrations_table(&db_with_name).await?;
      migration::Migrator::up(&db_with_name, None).await?;
    }
    eprintln!("Database reset and migrations applied.");
    return Ok(());
  }
  if run_migrations_only {
    if let Some(name) = db_name.as_deref() {
      ensure_database_charset(&db, name).await?;
    }
    ensure_default_storage_engine(&db).await?;
    ensure_migrations_table(&db).await?;
    migration::Migrator::up(&db, None).await?;
    eprintln!("Migrations applied.");
    return Ok(());
  }
  init_db(&db, db_name.as_deref()).await?;

  if run_seed_only {
    seed_db(&db).await?;
    eprintln!("Seed data applied.");
    return Ok(());
  }

  let state = AppState {
    client: reqwest::Client::new(),
    openai_api_key,
    openai_model,
    system_prompt,
    user_prompt,
    db,
  };

  let app = Router::new()
    .route("/healthz", get(health))
    .route("/ingest", post(ingest))
    .fallback(fallback_404)
    .layer(middleware::from_fn(method_not_allowed))
    .with_state(state)
    .layer(middleware::from_fn(log_requests));

  let addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:3005".to_string());
  eprintln!(
    "Server running on http://{addr} (db: {})",
    sanitize_db_url(&database_url)
  );
  let listener = tokio::net::TcpListener::bind(&addr).await?;
  axum::serve(listener, app).await?;

  Ok(())
}

async fn health() -> impl IntoResponse {
  StatusCode::OK
}

async fn fallback_404() -> impl IntoResponse {
  let body = Json(ErrorResponse {
    error: ErrorDetail {
      code: StatusCode::NOT_FOUND.as_u16() as i32,
      message: "HTTP ERROR 404 Not Found".to_string(),
    },
  });
  (StatusCode::NOT_FOUND, body)
}

async fn method_not_allowed(
  req: Request<axum::body::Body>,
  next: Next,
) -> Response {
  let method = req.method().clone();
  let response = next.run(req).await;
  if response.status() != StatusCode::METHOD_NOT_ALLOWED {
    return response;
  }
  let body = Json(ErrorResponse {
    error: ErrorDetail {
      code: StatusCode::METHOD_NOT_ALLOWED.as_u16() as i32,
      message: format!("HTTP ERROR 405 Method Not Allowed ({method})"),
    },
  });
  (StatusCode::METHOD_NOT_ALLOWED, body).into_response()
}

async fn log_requests(req: Request<axum::body::Body>, next: Next) -> Response {
  let method = req.method().clone();
  let uri = req.uri().clone();
  let start = Instant::now();
  let response = next.run(req).await;
  let status = response.status();
  eprintln!(
    "[{}] {} {} ({}ms)",
    status.as_u16(),
    method,
    uri,
    start.elapsed().as_millis()
  );
  response
}

async fn ingest(
  State(state): State<AppState>,
  headers: axum::http::HeaderMap,
  mut multipart: Multipart,
) -> Result<Json<IngestResponse>, (StatusCode, Json<ErrorResponse>)> {
  let mut image_bytes: Option<Vec<u8>> = None;
  let mut image_mime = "image/png".to_string();

  while let Some(field) = multipart
    .next_field()
    .await
    .map_err(internal_error("Failed to read multipart"))?
  {
    if field.name() == Some("file") {
      if let Some(content_type) = field.content_type() {
        image_mime = content_type.to_string();
      }
      let data = field
        .bytes()
        .await
        .map_err(internal_error("Failed to read upload bytes"))?;
      image_bytes = Some(data.to_vec());
      break;
    }
  }

  let image_bytes =
    image_bytes.ok_or_else(|| bad_request("Missing `file` field in multipart"))?;

  let user_id = require_user_id(&state.db, &headers).await?;
  let subscription_id = require_subscription(&state.db, &user_id).await?;

  eprintln!(
    "Ingest start user_id={} bytes={} mime={}",
    user_id,
    image_bytes.len(),
    image_mime
  );

  let file_name = save_image(&image_bytes, &image_mime).map_err(internal_error("Save image failed"))?;
  let record_id = insert_screen_result(&state.db, Some(&user_id), &file_name).await;

  match call_openai(&state, &image_bytes, &image_mime).await {
    Ok((response, raw_output)) => {
      let debug_json = serde_json::json!({
        "response": response.clone(),
        "raw": raw_output
      });
      decrement_subscription(&state.db, subscription_id).await?;
      update_screen_result(
        &state.db,
        &record_id,
        "DONE",
        &debug_json,
      )
      .await;
      eprintln!(
        "Ingest success user_id={} record_id={} file_name={}",
        user_id, record_id, file_name
      );
      Ok(Json(response))
    }
    Err((status, body)) => {
      let debug_json = serde_json::json!({
        "status": status.as_u16(),
        "error": body.error.clone()
      });
      update_screen_result(
        &state.db,
        &record_id,
        "ERROR",
        &debug_json,
      )
      .await;
      eprintln!(
        "Ingest error user_id={} record_id={} status={} error={:?}",
        user_id,
        record_id,
        status.as_u16(),
        body.error
      );
      Err((status, body))
    }
  }
}

async fn call_openai(
  state: &AppState,
  image_bytes: &[u8],
  image_mime: &str,
) -> Result<(IngestResponse, String), (StatusCode, Json<ErrorResponse>)> {
  let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);
  let image_url = format!("data:{image_mime};base64,{encoded}");

  let body = serde_json::json!({
    "model": state.openai_model,
    "input": [
      {
        "role": "system",
        "content": [
          { "type": "input_text", "text": state.system_prompt }
        ]
      },
      {
        "role": "user",
        "content": [
          { "type": "input_text", "text": state.user_prompt },
          { "type": "input_image", "image_url": image_url }
        ]
      }
    ],
    "tools": [
      {
        "type": "function",
        "name": "submit_solution",
        "description": "Return the final solution for the screenshot as structured data.",
        "parameters": {
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "language": { "type": "string" },
            "text": { "type": "string", "description": "Markdown explanation with step-by-step solution." },
            "code": { "type": "string", "description": "Code snippet(s) without markdown fences." }
          },
          "required": ["language", "text", "code"]
        },
        "strict": true
      }
    ],
    "tool_choice": { "type": "function", "name": "submit_solution" }
  });

  let response = state
    .client
    .post("https://api.openai.com/v1/responses")
    .bearer_auth(&state.openai_api_key)
    .json(&body)
    .send()
    .await
    .map_err(internal_error("OpenAI request failed"))?;

  if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    return Err(error_response(
      StatusCode::BAD_GATEWAY,
      &format!("OpenAI error: {status} {body}"),
      None,
    ));
  }

  let api: OpenAiResponse = response
    .json()
    .await
    .map_err(internal_error("Invalid OpenAI JSON response"))?;
  if let Some(tool) = extract_tool_call(&api) {
    let raw = serde_json::to_string(&tool).unwrap_or_default();
    let mut parsed = IngestResponse {
      text: tool.text,
      code: tool.code,
    };
    normalize_response(&mut parsed);
    return Ok((parsed, raw));
  }

  let output_text =
    extract_output_text(&api).ok_or_else(|| bad_gateway("Missing OpenAI output"))?;
  let mut parsed = serde_json::from_str::<IngestResponse>(&output_text).map_err(|err| {
    error_response(
      StatusCode::BAD_GATEWAY,
      &format!("Failed to parse model JSON: {err}"),
      None,
    )
  })?;

  normalize_response(&mut parsed);
  Ok((parsed, output_text))
}

fn extract_output_text(api: &OpenAiResponse) -> Option<String> {
  for item in &api.output {
    if item.r#type != "message" {
      continue;
    }
    if let Some(content) = &item.content {
      for part in content {
        if part.r#type == "output_text" {
          if let Some(text) = &part.text {
            return Some(text.clone());
          }
        }
      }
    }
  }
  None
}

fn extract_tool_call(api: &OpenAiResponse) -> Option<ToolResult> {
  for item in &api.output {
    if item.r#type != "function_call" {
      continue;
    }
    if item.name.as_deref() != Some("submit_solution") {
      continue;
    }
    if let Some(args) = &item.arguments {
      if let Ok(parsed) = serde_json::from_str::<ToolResult>(args) {
        return Some(parsed);
      }
    }
  }
  None
}

async fn insert_screen_result(
  db: &DatabaseConnection,
  user_id: Option<&str>,
  file_name: &str,
) -> String {
  use entity::screen_results;
  let id = Uuid::new_v4().to_string();
  let active = screen_results::ActiveModel {
    id: Set(id.clone()),
    user_id: Set(user_id.map(|s| s.to_string())),
    file_name: Set(file_name.to_string()),
    status: Set("RUNNING".to_string()),
    ..Default::default()
  };
  let _ = active.insert(db).await;
  id
}

fn normalize_response(response: &mut IngestResponse) {
  let code = response.code.trim();
  let placeholder = code.is_empty()
    || code.eq_ignore_ascii_case("rs")
    || code.eq_ignore_ascii_case("rust")
    || code.eq_ignore_ascii_case("code")
    || code.len() < 3;
  if !placeholder {
    return;
  }
  if let Some(extracted) = extract_fenced_code(&response.text) {
    if !extracted.trim().is_empty() {
      response.code = extracted;
      return;
    }
  }
  if std::env::var("FAUX_DEBUG_CODE_SAMPLE").ok().as_deref() == Some("1") {
    response.code = "fn main() {\n  println!(\"debug code sample\");\n}\n".to_string();
  }
}

fn extract_fenced_code(text: &str) -> Option<String> {
  let mut blocks: Vec<String> = Vec::new();
  let mut current: Vec<String> = Vec::new();
  let mut in_block = false;

  for line in text.lines() {
    let trimmed = line.trim_start();
    if trimmed.starts_with("```") {
      if in_block {
        blocks.push(current.join("\n"));
        current.clear();
        in_block = false;
      } else {
        in_block = true;
      }
      continue;
    }
    if in_block {
      current.push(line.to_string());
    }
  }

  if in_block && !current.is_empty() {
    blocks.push(current.join("\n"));
  }

  let joined = blocks.join("\n\n").trim().to_string();
  if joined.is_empty() {
    None
  } else {
    Some(joined)
  }
}

async fn update_screen_result(
  db: &DatabaseConnection,
  id: &str,
  status: &str,
  debug_json: &serde_json::Value,
) {
  use entity::screen_results;
  if let Ok(Some(model)) = screen_results::Entity::find_by_id(id.to_string()).one(db).await {
    let mut active: screen_results::ActiveModel = model.into();
    active.status = Set(status.to_string());
    active.debug = Set(Some(debug_json.clone()));
    let _ = active.update(db).await;
  }
}

async fn init_db(db: &DatabaseConnection, db_name: Option<&str>) -> Result<(), sea_orm::DbErr> {
  use entity::screen_results;
  let exists = screen_results::Entity::find().one(db).await?;
  if exists.is_none() {
    if let Some(name) = db_name {
      ensure_database_charset(db, name).await?;
    }
    ensure_default_storage_engine(db).await?;
    ensure_migrations_table(db).await?;
    migration::Migrator::up(db, None).await?;
  }
  Ok(())
}

fn bad_request(message: &str) -> (StatusCode, Json<ErrorResponse>) {
  error_response(StatusCode::BAD_REQUEST, message, None)
}

fn unauthorized(message: &str, code: Option<i32>) -> (StatusCode, Json<ErrorResponse>) {
  error_response(StatusCode::UNAUTHORIZED, message, code)
}

fn forbidden(message: &str, code: Option<i32>) -> (StatusCode, Json<ErrorResponse>) {
  error_response(StatusCode::FORBIDDEN, message, code)
}

fn bad_gateway(message: &str) -> (StatusCode, Json<ErrorResponse>) {
  error_response(StatusCode::BAD_GATEWAY, message, None)
}

fn error_response(
  status: StatusCode,
  message: &str,
  code: Option<i32>,
) -> (StatusCode, Json<ErrorResponse>) {
  (
    status,
    Json(ErrorResponse {
      error: ErrorDetail {
        code: code.unwrap_or(status.as_u16() as i32),
        message: message.to_string(),
      },
    }),
  )
}

fn internal_error<E: std::fmt::Display>(
  message: &str,
) -> impl FnOnce(E) -> (StatusCode, Json<ErrorResponse>) + '_ {
  move |err| {
    (
      StatusCode::INTERNAL_SERVER_ERROR,
      Json(ErrorResponse {
        error: ErrorDetail {
          code: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
          message: format!("{message}: {err}"),
        },
      }),
    )
  }
}

fn save_image(bytes: &[u8], mime: &str) -> Result<String, std::io::Error> {
  let ext = if mime.contains("png") {
    "png"
  } else if mime.contains("jpeg") || mime.contains("jpg") {
    "jpg"
  } else {
    "bin"
  };
  let file_name = format!("{}.{}", Uuid::new_v4(), ext);
  let dir = std::path::Path::new("data/images");
  std::fs::create_dir_all(dir)?;
  let path = dir.join(&file_name);
  std::fs::write(path, bytes)?;
  Ok(file_name)
}

async fn require_user_id(
  db: &DatabaseConnection,
  headers: &axum::http::HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
  let auth = headers
    .get(axum::http::header::AUTHORIZATION)
    .and_then(|v| v.to_str().ok())
    .unwrap_or("")
    .trim()
    .to_string();
  let token = auth.strip_prefix("Bearer ").unwrap_or(&auth).trim();
  if token.is_empty() {
    return Err(unauthorized("API key was not provided", Some(100)));
  }

  let stmt = Statement::from_sql_and_values(
    DatabaseBackend::MySql,
    "SELECT user_id FROM `keys` WHERE `key` = ? LIMIT 1",
    vec![Value::from(token.to_string())],
  );
  let row = db
    .query_one(stmt)
    .await
    .map_err(internal_error("DB error"))?;
  if let Some(row) = row {
    if let Ok(Some(user_id)) = row.try_get::<Option<String>>("", "user_id") {
      return Ok(user_id);
    }
  }
  Err(unauthorized("Invalid API key", Some(100)))
}

async fn require_subscription(
  db: &DatabaseConnection,
  user_id: &str,
) -> Result<i64, (StatusCode, Json<ErrorResponse>)> {
  let stmt = Statement::from_sql_and_values(
    DatabaseBackend::MySql,
    "SELECT id, credits FROM subscriptions \
     WHERE user_id = ? AND (expires_at IS NULL OR expires_at > NOW()) \
     ORDER BY expires_at DESC LIMIT 1",
    vec![Value::from(user_id.to_string())],
  );
  let row = db
    .query_one(stmt)
    .await
    .map_err(internal_error("DB error"))?;
  let Some(row) = row else {
    return Err(forbidden("No active subscription", None));
  };
  let id: i64 = row.try_get("", "id").unwrap_or(0);
  let credits: i64 = row.try_get("", "credits").unwrap_or(0);
  if credits <= 0 {
    return Err(forbidden("No credits available", Some(101)));
  }
  Ok(id)
}

async fn decrement_subscription(
  db: &DatabaseConnection,
  subscription_id: i64,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
  let stmt = Statement::from_sql_and_values(
    DatabaseBackend::MySql,
    "UPDATE subscriptions SET credits = credits - 1 WHERE id = ? AND credits > 0",
    vec![Value::from(subscription_id)],
  );
  let result = db
    .execute(stmt)
    .await
    .map_err(internal_error("DB error"))?;
  if result.rows_affected() == 0 {
    return Err(forbidden("No credits available", Some(101)));
  }
  Ok(())
}

async fn seed_db(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
  let users = [
    (
      "0f2d5b6a-7f8a-4b2e-9b43-0a5a2a9f9b01",
      "user@example.com",
      "changeme",
      "Test",
      "User",
      1,
      "user",
      10,
      "USER-TEST-1",
      "1234",
      "3e1f3f5a-2f1b-4e06-9e2a-7a43d2c5c101",
      "4f2a5e0e-6f92-4b8e-a7f8-873d0a1a1101",
      "5b0e5a1b-4b18-4a6e-bb7c-5f95c9c5b101",
      "6a2b5c3d-7e8f-4a9b-8c0d-1e2f3a4b5101",
      3,
      10,
    ),
    (
      "1a7b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c02",
      "admin@example.com",
      "changeme",
      "Test",
      "Admin",
      1,
      "admin",
      100,
      "ADMIN-TEST-1",
      "5678",
      "7b3c6d4e-8f90-4a1b-9c2d-3e4f5a6b5202",
      "8c4d7e5f-9012-4b3c-ad4e-5f6a7b8c6202",
      "9d5e8f60-0123-4c5d-be6f-7a8b9c0d7202",
      "ae6f9071-1234-4d6e-cf70-8b9c0d1e8202",
      1,
      200,
    ),
    (
      "2b3c4d5e-6f70-8192-a3b4-c5d6e7f80903",
      "mod@example.com",
      "changeme",
      "Test",
      "Mod",
      1,
      "mod",
      50,
      "MOD-TEST-1",
      "2468",
      "bf70a182-2345-4e7f-d081-9c0d1e2f9303",
      "c081b293-3456-4f80-e192-ad1e2f3a0403",
      "d192c3a4-4567-4081-f2a3-be2f3a4b1403",
      "e2a3d4b5-5678-4182-03b4-cf3a4b5c2403",
      2,
      50,
    ),
  ];

  let insert_free = sea_orm::Statement::from_string(
    sea_orm::DatabaseBackend::MySql,
    "INSERT IGNORE INTO packages (id, name, rate_limit) VALUES (1, 'Free', 60)".to_string(),
  );
  let insert_pro = sea_orm::Statement::from_string(
    sea_orm::DatabaseBackend::MySql,
    "INSERT IGNORE INTO packages (id, name, rate_limit) VALUES (2, 'Pro', 600)".to_string(),
  );
  db.execute(insert_free).await?;
  db.execute(insert_pro).await?;

  for (
    id,
    email,
    password,
    first_name,
    last_name,
    confirmd,
    role_name,
    elevation,
    code,
    pin,
    role_id,
    link_id,
    key_id,
    settings_id,
    subscription_id,
    credits,
  ) in users
  {
    let insert_user = sea_orm::Statement::from_string(
      sea_orm::DatabaseBackend::MySql,
      format!(
        "INSERT IGNORE INTO users (id, email, password, first_name, last_name, confirmd) \
         VALUES ('{id}', '{email}', '{password}', '{first_name}', '{last_name}', {confirmd})"
      ),
    );
    db.execute(insert_user).await?;

    let insert_role = sea_orm::Statement::from_string(
      sea_orm::DatabaseBackend::MySql,
      format!(
        "INSERT IGNORE INTO roles (id, user_id, name, elevation) \
         VALUES ('{role_id}', '{id}', '{role_name}', {elevation})"
      ),
    );
    db.execute(insert_role).await?;

    let insert_link = sea_orm::Statement::from_string(
      sea_orm::DatabaseBackend::MySql,
      format!(
        "INSERT IGNORE INTO links (id, user_id, code, pin) \
         VALUES ('{link_id}', '{id}', '{code}', '{pin}')"
      ),
    );
    db.execute(insert_link).await?;

    let api_key = format!("jwt-{id}");
    let insert_key = sea_orm::Statement::from_string(
      sea_orm::DatabaseBackend::MySql,
      format!(
        "INSERT IGNORE INTO `keys` (id, user_id, name, `key`) \
         VALUES ('{key_id}', '{id}', 'default', '{api_key}')"
      ),
    );
    db.execute(insert_key).await?;

    let config_json = serde_json::json!({
      "test": true,
      "main_position": { "x": 1113.0, "y": 284.0 },
      "api_key": ""
    });
    let insert_settings = sea_orm::Statement::from_string(
      sea_orm::DatabaseBackend::MySql,
      format!(
        "INSERT IGNORE INTO settings (id, user_id, name, config) \
         VALUES ('{settings_id}', '{id}', 'default', '{}')",
        config_json.to_string().replace('\'', "''")
      ),
    );
    db.execute(insert_settings).await?;

    let package_id = if role_name == "admin" { 2 } else { 1 };
    let insert_subscription = sea_orm::Statement::from_string(
      sea_orm::DatabaseBackend::MySql,
      format!(
        "INSERT IGNORE INTO subscriptions (id, user_id, package_id, expires_at, credits) \
         VALUES ({subscription_id}, '{id}', {package_id}, DATE_ADD(NOW(), INTERVAL 30 DAY), {credits})"
      ),
    );
    db.execute(insert_subscription).await?;
  }
  Ok(())
}

fn sanitize_db_url(url: &str) -> String {
  let Some(scheme_idx) = url.find("://") else {
    return url.to_string();
  };
  let (scheme, rest) = url.split_at(scheme_idx + 3);
  let Some(at_idx) = rest.find('@') else {
    return url.to_string();
  };
  let (creds, host) = rest.split_at(at_idx);
  if let Some(colon_idx) = creds.find(':') {
    let user = &creds[..colon_idx];
    return format!("{scheme}{user}:***{host}");
  }
  format!("{scheme}{creds}{host}")
}

fn database_name_from_url(url: &str) -> Option<String> {
  let without_params = url.split('?').next().unwrap_or(url);
  let name = without_params.rsplit('/').next()?;
  if name.is_empty() {
    None
  } else {
    Some(name.to_string())
  }
}

fn database_url_with_db(url: &str, db_name: &str) -> String {
  let base = url.split('?').next().unwrap_or(url);
  let mut parts = base.rsplitn(2, '/');
  let _ = parts.next();
  let head = parts.next().unwrap_or(base);
  format!("{}/{}", head, db_name)
}

async fn ensure_database_charset(
  db: &DatabaseConnection,
  db_name: &str,
) -> Result<(), sea_orm::DbErr> {
  db
    .execute_unprepared(&format!(
      "ALTER DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_bin",
      db_name
    ))
    .await?;
  Ok(())
}

async fn ensure_default_storage_engine(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
  db
    .execute_unprepared("SET SESSION default_storage_engine=InnoDB")
    .await?;
  Ok(())
}

async fn ensure_migrations_table(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
  let sql = r#"
    CREATE TABLE IF NOT EXISTS seaql_migrations (
      version VARCHAR(255) NOT NULL PRIMARY KEY,
      applied_at BIGINT NOT NULL
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin
  "#;
  db.execute_unprepared(sql).await?;
  Ok(())
}

async fn reset_database(db: &DatabaseConnection, db_name: &str) -> Result<(), sea_orm::DbErr> {
  db
    .execute_unprepared(&format!("DROP DATABASE IF EXISTS `{}`", db_name))
    .await?;
  db
    .execute_unprepared(&format!(
      "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_bin",
      db_name
    ))
    .await?;
  Ok(())
}
