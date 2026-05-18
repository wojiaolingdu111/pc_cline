use serde::{Deserialize, Serialize};
use tauri::State;
use crate::license::{LicenseInfo, ACTIVATE_URL};
use crate::python_service::{
    self, CloneVoicePayload, GenerateSpeechPayload, ServiceStatusResponse, VoiceProfile,
};
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoicesPayload {
    builtin_voices: Vec<VoiceProfile>,
    custom_voices: Vec<VoiceProfile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSpeechArgs {
    pub payload: GenerateSpeechArgsPayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSpeechArgsPayload {
    pub text: String,
    pub voice_id: String,
    pub speed: f32,
    pub language: String,
    pub output_format: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneVoiceArgs {
    pub payload: CloneVoiceArgsPayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneVoiceArgsPayload {
    pub name: String,
    pub audio_path: String,
    pub language: String,
}

#[tauri::command]
pub async fn get_service_status(state: State<'_, AppState>) -> Result<ServiceStatusResponse, String> {
    Ok(
        python_service::get_service_status(
            &state.client,
            &state.python_base_url,
            state.backend_token.as_deref(),
        )
        .await,
    )
}

#[tauri::command]
pub async fn list_voices(state: State<'_, AppState>) -> Result<VoicesPayload, String> {
    let response = python_service::list_voices(
        &state.client,
        &state.python_base_url,
        state.backend_token.as_deref(),
    )
    .await
    .map_err(|error| error.to_string())?;

    Ok(VoicesPayload {
        builtin_voices: response.builtin_voices,
        custom_voices: response.custom_voices,
    })
}

#[tauri::command]
pub async fn generate_speech(
    payload: GenerateSpeechArgsPayload,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let response = python_service::generate_speech(
        &state.client,
        &state.python_base_url,
        state.backend_token.as_deref(),
        &GenerateSpeechPayload {
            text: payload.text,
            voice_id: payload.voice_id,
            speed: payload.speed,
            language: payload.language,
            output_format: payload.output_format,
        },
    )
    .await
    .map_err(|error| error.to_string())?;

    serde_json::to_value(response).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clone_voice(
    payload: CloneVoiceArgsPayload,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let response = python_service::clone_voice(
        &state.client,
        &state.python_base_url,
        state.backend_token.as_deref(),
        &CloneVoicePayload {
            name: payload.name,
            audio_path: payload.audio_path,
            language: payload.language,
        },
    )
    .await
    .map_err(|error| error.to_string())?;

    serde_json::to_value(response).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_voice_profile(
    voice_profile_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    python_service::delete_voice_profile(
        &state.client,
        &state.python_base_url,
        state.backend_token.as_deref(),
        &voice_profile_id,
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn pick_audio_file() -> Option<String> {
    tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .add_filter("Audio", &["wav", "mp3", "flac", "m4a", "ogg"])
            .set_title("选择参考音频文件（建议 6 秒以上清晰录音）")
            .pick_file()
            .map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
pub fn get_license_status(state: State<'_, AppState>) -> LicenseInfo {
    state.license.lock().unwrap().get_info()
}

#[tauri::command]
pub async fn activate_license(
    key: String,
    state: State<'_, AppState>,
) -> Result<LicenseInfo, String> {
    let machine_id = state.license.lock().unwrap().machine_id();

    // 联网验证
    let client = &state.client;
    let resp = client
        .post(ACTIVATE_URL)
        .json(&serde_json::json!({ "license_key": key, "machine_id": machine_id }))
        .send()
        .await
        .map_err(|e| format!("无法连接授权服务器: {}", e))?;

    let result = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("授权服务器响应异常: {}", e))?;

    if result.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
        state
            .license
            .lock()
            .unwrap()
            .set_license_key(key)
            .map_err(|e| format!("保存授权信息失败: {}", e))?;
        Ok(state.license.lock().unwrap().get_info())
    } else {
        Err(result
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("授权码无效")
            .to_string())
    }
}
