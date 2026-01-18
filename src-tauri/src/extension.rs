//! Local HTTP bridge for the browser extension integration.
//!
//! The server is bound to 127.0.0.1 and guarded by a shared token. It exposes
//! endpoints for matching entries by URL and retrieving secrets for autofill.

use crate::models::{AppState, Entry, ExtensionConfig};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::thread;
use tauri::{AppHandle, Manager};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use url::{form_urlencoded, Url};
use zeroize::Zeroize;

const EXTENSION_CONFIG_FILENAME: &str = "extension.json";

#[derive(Serialize)]
struct ExtensionEntry {
  id: String,
  title: String,
  username: String,
  url: String,
}

impl From<&Entry> for ExtensionEntry {
  fn from(entry: &Entry) -> Self {
    Self {
      id: entry.id.clone(),
      title: entry.title.clone(),
      username: entry.username.clone(),
      url: entry.url.clone(),
    }
  }
}

fn extension_config_path(app: &AppHandle) -> Result<PathBuf, String> {
  let dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("app_data_dir failed: {e}"))?;
  fs::create_dir_all(&dir).map_err(|e| format!("create_dir_all failed: {e}"))?;
  Ok(dir.join(EXTENSION_CONFIG_FILENAME))
}

pub fn load_or_create_config(app: &AppHandle) -> Result<ExtensionConfig, String> {
  let path = extension_config_path(app)?;
  if path.exists() {
    let raw = fs::read_to_string(&path).map_err(|e| format!("read extension config failed: {e}"))?;
    let mut config: ExtensionConfig =
      serde_json::from_str(&raw).map_err(|e| format!("parse extension config failed: {e}"))?;
    if config.token.trim().is_empty() {
      config.token = ExtensionConfig::new().token;
      save_config(app, &config)?;
    }
    Ok(config)
  } else {
    let config = ExtensionConfig::new();
    save_config(app, &config)?;
    Ok(config)
  }
}

pub fn save_config(app: &AppHandle, config: &ExtensionConfig) -> Result<(), String> {
  let path = extension_config_path(app)?;
  let serialized =
    serde_json::to_string_pretty(config).map_err(|e| format!("serialize extension config failed: {e}"))?;
  fs::write(&path, serialized).map_err(|e| format!("write extension config failed: {e}"))?;
  Ok(())
}

pub fn start_extension_server(_app: &AppHandle, state: AppState) {
  let port = match state.extension_config.lock() {
    Ok(cfg) => cfg.port,
    Err(_) => {
      eprintln!("extension server: extension config mutex poisoned");
      return;
    }
  };
  let address = format!("127.0.0.1:{port}");
  let server = match Server::http(&address) {
    Ok(server) => server,
    Err(e) => {
      eprintln!("extension server: failed to bind {address}: {e}");
      return;
    }
  };

  thread::spawn(move || {
    for request in server.incoming_requests() {
      handle_request(&state, request);
    }
  });
}

fn handle_request(state: &AppState, request: Request) {
  if *request.method() == Method::Options {
    respond_json(request, StatusCode(204), json!({}));
    return;
  }

  let (path, query) = split_path_query(request.url());

  match (request.method(), path) {
    (&Method::Get, "/v1/status") => {
      if let Err(err) = ensure_authorized(state, &request) {
        respond_auth_error(request, err);
        return;
      }
      state.heartbeat();
      let locked = is_locked(state);
      respond_json(request, StatusCode(200), json!({ "locked": locked }));
    }
    (&Method::Get, "/v1/entries") => {
      if let Err(err) = ensure_authorized(state, &request) {
        respond_auth_error(request, err);
        return;
      }
      state.heartbeat();
      if is_locked(state) {
        respond_json(
          request,
          StatusCode(423),
          json!({ "error": "vault is locked" }),
        );
        return;
      }
      let params = parse_query(query);
      let target_url = match params.get("url") {
        Some(value) if !value.trim().is_empty() => value,
        _ => {
          respond_json(
            request,
            StatusCode(400),
            json!({ "error": "url is required" }),
          );
          return;
        }
      };

      let target_host = match normalize_host(target_url) {
        Some(host) => host,
        None => {
          respond_json(
            request,
            StatusCode(400),
            json!({ "error": "invalid url" }),
          );
          return;
        }
      };

      let entries_guard = match state.entries.lock() {
        Ok(g) => g,
        Err(_) => {
          respond_json(
            request,
            StatusCode(500),
            json!({ "error": "entries mutex poisoned" }),
          );
          return;
        }
      };
      let entries = match entries_guard.as_ref() {
        Some(entries) => entries,
        None => {
          respond_json(
            request,
            StatusCode(423),
            json!({ "error": "vault is locked" }),
          );
          return;
        }
      };

      let matches: Vec<ExtensionEntry> = entries
        .iter()
        .filter_map(|entry| {
          let entry_host = normalize_host(entry.url.as_str())?;
          if host_matches(&entry_host, &target_host) {
            Some(ExtensionEntry::from(entry))
          } else {
            None
          }
        })
        .collect();

      respond_json(request, StatusCode(200), json!({ "entries": matches }));
    }
    (&Method::Get, "/v1/secret") => {
      if let Err(err) = ensure_authorized(state, &request) {
        respond_auth_error(request, err);
        return;
      }
      state.heartbeat();
      if is_locked(state) {
        respond_json(
          request,
          StatusCode(423),
          json!({ "error": "vault is locked" }),
        );
        return;
      }
      let params = parse_query(query);
      let entry_id = match params.get("id") {
        Some(value) if !value.trim().is_empty() => value,
        _ => {
          respond_json(
            request,
            StatusCode(400),
            json!({ "error": "id is required" }),
          );
          return;
        }
      };

      let entries_guard = match state.entries.lock() {
        Ok(g) => g,
        Err(_) => {
          respond_json(
            request,
            StatusCode(500),
            json!({ "error": "entries mutex poisoned" }),
          );
          return;
        }
      };
      let entries = match entries_guard.as_ref() {
        Some(entries) => entries,
        None => {
          respond_json(
            request,
            StatusCode(423),
            json!({ "error": "vault is locked" }),
          );
          return;
        }
      };

      let mut secret = match entries.iter().find(|entry| entry.id == *entry_id) {
        Some(entry) => entry.password.clone(),
        None => {
          respond_json(
            request,
            StatusCode(404),
            json!({ "error": "entry not found" }),
          );
          return;
        }
      };

      let payload = json!({ "password": secret });
      secret.zeroize();
      respond_json(request, StatusCode(200), payload);
    }
    _ => {
      respond_json(request, StatusCode(404), json!({ "error": "not found" }));
    }
  }
}

fn split_path_query(url: &str) -> (&str, Option<&str>) {
  match url.split_once('?') {
    Some((path, query)) => (path, Some(query)),
    None => (url, None),
  }
}

fn parse_query(query: Option<&str>) -> HashMap<String, String> {
  match query {
    Some(value) => form_urlencoded::parse(value.as_bytes())
      .into_owned()
      .collect(),
    None => HashMap::new(),
  }
}

fn normalize_host(raw: &str) -> Option<String> {
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return None;
  }
  let candidate = if trimmed.contains("://") {
    trimmed.to_string()
  } else {
    format!("https://{trimmed}")
  };
  Url::parse(&candidate)
    .ok()
    .and_then(|url| url.host_str().map(|host| host.to_lowercase()))
}

fn host_matches(entry_host: &str, target_host: &str) -> bool {
  let entry = entry_host.strip_prefix("www.").unwrap_or(entry_host);
  let target = target_host.strip_prefix("www.").unwrap_or(target_host);
  if entry == target {
    return true;
  }
  target.ends_with(&format!(".{entry}"))
}

fn is_locked(state: &AppState) -> bool {
  match state.session.lock() {
    Ok(guard) => guard.is_none(),
    Err(_) => true,
  }
}

#[derive(Debug)]
enum AuthError {
  Disabled,
  Missing,
  Invalid,
}

fn ensure_authorized(state: &AppState, request: &Request) -> Result<(), AuthError> {
  let config = state.extension_config.lock().map_err(|_| AuthError::Disabled)?;
  if !config.enabled {
    return Err(AuthError::Disabled);
  }
  let token = request_token(request).ok_or(AuthError::Missing)?;
  if token != config.token {
    return Err(AuthError::Invalid);
  }
  Ok(())
}

fn request_token(request: &Request) -> Option<String> {
  if let Some(value) = header_value(request, "X-Organizer-Token") {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
      return Some(trimmed.to_string());
    }
  }
  if let Some(value) = header_value(request, "Authorization") {
    let trimmed = value.trim();
    if let Some(token) = trimmed.strip_prefix("Bearer ") {
      return Some(token.trim().to_string());
    }
  }
  None
}

fn header_value(request: &Request, name: &str) -> Option<String> {
  request
    .headers()
    .iter()
    .find(|header| header.field.equiv(name))
    .map(|header| header.value.as_str().to_string())
}

fn respond_auth_error(request: Request, err: AuthError) {
  let (status, message) = match err {
    AuthError::Disabled => (StatusCode(423), "extension disabled"),
    AuthError::Missing => (StatusCode(401), "missing token"),
    AuthError::Invalid => (StatusCode(401), "invalid token"),
  };
  respond_json(request, status, json!({ "error": message }));
}

fn respond_json(request: Request, status: StatusCode, body: serde_json::Value) {
  let payload = body.to_string();
  let response = Response::from_string(payload)
    .with_status_code(status)
    .with_header(header("Content-Type", "application/json"))
    .with_header(header("Access-Control-Allow-Origin", "*"))
    .with_header(header(
      "Access-Control-Allow-Headers",
      "Authorization, Content-Type, X-Organizer-Token",
    ))
    .with_header(header(
      "Access-Control-Allow-Methods",
      "GET, POST, OPTIONS",
    ));
  let _ = request.respond(response);
}

fn header(name: &str, value: &str) -> Header {
  Header::from_bytes(name, value).unwrap()
}
