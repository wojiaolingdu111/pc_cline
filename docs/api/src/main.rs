use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use rusqlite::Connection;
use rust_embed::Embed;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tracing_subscriber::EnvFilter;

type Db = Arc<Mutex<Connection>>;
type ApiResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

#[derive(Embed)]
#[folder = "admin/"]
struct AdminAssets;

fn json_err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({"error": msg})))
}

fn check_auth(headers: &HeaderMap) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let password = std::env::var("ADMIN_PASSWORD").unwrap_or_default();
    if password.is_empty() {
        return Err(json_err(StatusCode::INTERNAL_SERVER_ERROR, "ADMIN_PASSWORD 未配置"));
    }
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if token != password {
        return Err(json_err(StatusCode::UNAUTHORIZED, "密码错误"));
    }
    Ok(())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn init_db(db_path: &str) -> rusqlite::Result<Connection> {
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS licenses (
            code TEXT PRIMARY KEY,
            max_activations INTEGER DEFAULT 1,
            activation_count INTEGER DEFAULT 0,
            machine_id TEXT,
            activated_at INTEGER,
            expires_at INTEGER
        )",
    )?;
    Ok(conn)
}

fn query_license(
    db: &Connection,
    code: &str,
) -> rusqlite::Result<Option<serde_json::Value>> {
    let mut stmt = db.prepare(
        "SELECT code, max_activations, activation_count, machine_id, activated_at, expires_at
         FROM licenses WHERE code = ?",
    )?;
    let mut rows = stmt.query([code])?;
    match rows.next()? {
        Some(row) => Ok(Some(serde_json::json!({
            "code": row.get::<_, String>(0)?,
            "max_activations": row.get::<_, i32>(1)?,
            "activation_count": row.get::<_, i32>(2)?,
            "machine_id": row.get::<_, Option<String>>(3)?,
            "activated_at": row.get::<_, Option<i64>>(4)?,
            "expires_at": row.get::<_, Option<i64>>(5)?,
        }))),
        None => Ok(None),
    }
}

// ---- Admin Handlers ----

async fn handle_list(
    State(db): State<Db>,
    headers: HeaderMap,
) -> ApiResult {
    check_auth(&headers)?;

    let db = db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT code, max_activations, activation_count, machine_id, activated_at, expires_at FROM licenses")
        .map_err(|_| json_err(StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "code": row.get::<_, String>(0)?,
                "max_activations": row.get::<_, i32>(1)?,
                "activation_count": row.get::<_, i32>(2)?,
                "machine_id": row.get::<_, Option<String>>(3)?,
                "activated_at": row.get::<_, Option<i64>>(4)?,
                "expires_at": row.get::<_, Option<i64>>(5)?,
            }))
        })
        .map_err(|_| json_err(StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误"))?;

    let mut list = Vec::new();
    for row in rows {
        if let Ok(item) = row {
            list.push(item);
        }
    }

    Ok(Json(serde_json::json!({"keys": list})))
}

#[derive(Deserialize)]
struct AddBody {
    code: String,
    max_activations: Option<i32>,
    expires_in_days: Option<i64>,
}

async fn handle_add(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(body): Json<AddBody>,
) -> ApiResult {
    check_auth(&headers)?;

    if body.code.is_empty() {
        return Err(json_err(StatusCode::BAD_REQUEST, "缺少授权码"));
    }

    let db = db.lock().unwrap();
    if query_license(&db, &body.code)
        .map_err(|_| json_err(StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误"))?
        .is_some()
    {
        return Err(json_err(StatusCode::BAD_REQUEST, "授权码已存在"));
    }

    let max_acts = body.max_activations.unwrap_or(1).max(1);
    let expires_at = body.expires_in_days.filter(|d| *d > 0).map(|d| now_ms() + d * 86400_000);

    db.execute(
        "INSERT INTO licenses (code, max_activations, expires_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![body.code, max_acts, expires_at],
    )
    .map_err(|_| json_err(StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误"))?;

    Ok(Json(serde_json::json!({"success": true, "code": body.code})))
}

#[derive(Deserialize)]
struct CodeBody {
    code: String,
}

async fn handle_remove(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(body): Json<CodeBody>,
) -> ApiResult {
    check_auth(&headers)?;

    if body.code.is_empty() {
        return Err(json_err(StatusCode::BAD_REQUEST, "缺少授权码"));
    }

    let db = db.lock().unwrap();
    if query_license(&db, &body.code)
        .map_err(|_| json_err(StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误"))?
        .is_none()
    {
        return Err(json_err(StatusCode::NOT_FOUND, "授权码不存在"));
    }

    db.execute("DELETE FROM licenses WHERE code = ?1", rusqlite::params![body.code])
        .map_err(|_| json_err(StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误"))?;

    Ok(Json(serde_json::json!({"success": true})))
}

#[derive(Deserialize)]
struct ExpireBody {
    code: String,
    days: u64,
}

async fn handle_expire(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(body): Json<ExpireBody>,
) -> ApiResult {
    check_auth(&headers)?;

    if body.code.is_empty() {
        return Err(json_err(StatusCode::BAD_REQUEST, "缺少授权码"));
    }

    let db = db.lock().unwrap();
    if query_license(&db, &body.code)
        .map_err(|_| json_err(StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误"))?
        .is_none()
    {
        return Err(json_err(StatusCode::NOT_FOUND, "授权码不存在"));
    }

    let expires_at = (body.days > 0).then(|| now_ms() + body.days as i64 * 86400_000);

    db.execute(
        "UPDATE licenses SET expires_at = ?1 WHERE code = ?2",
        rusqlite::params![expires_at, body.code],
    )
    .map_err(|_| json_err(StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误"))?;

    Ok(Json(serde_json::json!({"success": true, "expires_at": expires_at})))
}

// ---- License Handlers ----

#[derive(Deserialize)]
struct LicenseBody {
    license_key: String,
    machine_id: Option<String>,
}

fn verify_record(
    record: &serde_json::Value,
    machine_id: Option<&str>,
) -> ApiResult {
    if let Some(exp) = record["expires_at"].as_i64() {
        if now_ms() > exp {
            return Ok(Json(serde_json::json!({"valid": false, "message": "授权码已过期"})));
        }
    }

    if let Some(mid) = record["machine_id"].as_str() {
        if let Some(req_mid) = machine_id {
            if mid != req_mid {
                return Ok(Json(serde_json::json!({"valid": false, "message": "授权码已被其他设备绑定"})));
            }
        }
    }

    Ok(Json(serde_json::json!({
        "valid": true,
        "message": "授权码有效",
        "expires_at": record["expires_at"],
    })))
}

async fn handle_verify(
    State(db): State<Db>,
    Json(body): Json<LicenseBody>,
) -> ApiResult {
    if body.license_key.is_empty() {
        return Err(json_err(StatusCode::BAD_REQUEST, "缺少授权码"));
    }

    let db = db.lock().unwrap();
    let record = query_license(&db, &body.license_key)
        .map_err(|_| json_err(StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误"))?;

    match record {
        Some(r) => verify_record(&r, body.machine_id.as_deref()),
        None => Ok(Json(serde_json::json!({"valid": false, "message": "授权码无效"}))),
    }
}

async fn handle_activate(
    State(db): State<Db>,
    Json(body): Json<LicenseBody>,
) -> ApiResult {
    if body.license_key.is_empty() {
        return Err(json_err(StatusCode::BAD_REQUEST, "缺少授权码"));
    }
    let machine_id = body.machine_id.as_deref().unwrap_or("");
    if machine_id.is_empty() {
        return Err(json_err(StatusCode::BAD_REQUEST, "缺少机器标识"));
    }

    let db = db.lock().unwrap();
    let record = query_license(&db, &body.license_key)
        .map_err(|_| json_err(StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误"))?;

    let record = match record {
        Some(r) => r,
        None => return Ok(Json(serde_json::json!({"valid": false, "message": "授权码无效"}))),
    };

    if let Some(exp) = record["expires_at"].as_i64() {
        if now_ms() > exp {
            return Ok(Json(serde_json::json!({"valid": false, "message": "授权码已过期"})));
        }
    }

    if let Some(mid) = record["machine_id"].as_str() {
        if mid != machine_id {
            return Ok(Json(serde_json::json!({"valid": false, "message": "授权码已被其他设备绑定"})));
        }
        // already activated on this machine
        return Ok(Json(serde_json::json!({
            "valid": true, "message": "激活成功", "expires_at": record["expires_at"]
        })));
    }

    if record["activation_count"].as_i64().unwrap_or(0) >= record["max_activations"].as_i64().unwrap_or(1) {
        return Ok(Json(serde_json::json!({"valid": false, "message": "授权码激活次数已用完"})));
    }

    let now = now_ms();
    let affected = db
        .execute(
            "UPDATE licenses SET machine_id = ?1, activated_at = ?2, activation_count = activation_count + 1
             WHERE code = ?3 AND machine_id IS NULL",
            rusqlite::params![machine_id, now, body.license_key],
        )
        .map_err(|_| json_err(StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误"))?;

    if affected == 0 {
        return Ok(Json(serde_json::json!({"valid": false, "message": "授权码激活失败"})));
    }

    Ok(Json(serde_json::json!({
        "valid": true, "message": "激活成功", "expires_at": record["expires_at"]
    })))
}

// ---- Admin Frontend ----

async fn serve_admin(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match AdminAssets::get(path) {
        Some(file) => {
            let mime = match path.rsplit('.').next() {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "application/javascript; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("png") => "image/png",
                Some("svg") => "image/svg+xml",
                Some("ico") => "image/x-icon",
                _ => "application/octet-stream",
            };
            (
                [(axum::http::header::CONTENT_TYPE, mime)],
                file.data.to_vec(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

async fn serve_admin_root() -> impl IntoResponse {
    serve_admin(Path("index.html".into())).await
}

// ---- Main ----

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| {
        let exe = std::env::current_exe().unwrap();
        exe.parent()
            .unwrap()
            .join("data")
            .join("licenses.db")
            .to_string_lossy()
            .to_string()
    });

    let conn = init_db(&db_path).expect("数据库初始化失败");
    let db: Db = Arc::new(Mutex::new(conn));

    let app = Router::new()
        .route("/api/admin/list", get(handle_list))
        .route("/api/admin/add", post(handle_add))
        .route("/api/admin/remove", post(handle_remove))
        .route("/api/admin/expire", post(handle_expire))
        .route("/api/license/verify", post(handle_verify))
        .route("/api/license/activate", post(handle_activate))
        .route("/admin/{*path}", get(serve_admin))
        .route("/admin/", get(serve_admin_root))
        .with_state(db);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".into());
    tracing::info!("Server running on http://localhost:{}", port);
    tracing::info!("Admin: http://localhost:{}/admin/", port);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("无法绑定端口");

    axum::serve(listener, app).await.unwrap();
}
