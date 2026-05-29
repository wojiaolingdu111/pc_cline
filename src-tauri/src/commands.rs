use qwen3_tts_rs::{Language, Speaker, GenerationParams};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::license::{LicenseInfo, ACTIVATE_URL};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub running: bool,
    pub mode: String,
    pub model_loaded: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub voice_type: String,
    pub language: Vec<String>,
    pub preview_audio: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoicesPayload {
    pub builtin_voices: Vec<VoiceProfile>,
    pub custom_voices: Vec<VoiceProfile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSpeechResult {
    pub task_id: String,
    pub status: String,
    pub audio_path: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneVoiceResult {
    pub voice_profile_id: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Payload types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSpeechPayload {
    pub text: String,
    pub voice_id: String,
    pub speed: f64,
    pub language: String,
    pub output_format: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneVoicePayload {
    pub name: String,
    pub audio_path: String,
    pub language: String,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_service_status(state: State<'_, AppState>) -> ServiceStatus {
    let engine = state.tts_engine.lock().unwrap();
    ServiceStatus {
        running: engine.is_loaded(),
        mode: "qwen3".to_string(),
        model_loaded: engine.is_loaded(),
        message: if engine.is_loaded() {
            "模型已加载".to_string()
        } else if let Some(err) = engine.load_error() {
            format!("模型加载失败: {err}")
        } else {
            "模型未加载，请将 Qwen3-TTS 模型文件放入 models/ 目录".to_string()
        },
    }
}

#[tauri::command]
pub async fn list_voices(state: State<'_, AppState>) -> VoicesPayload {
    let mut engine = state.tts_engine.lock().unwrap();

    // Built-in voices from the loaded model
    let builtin_voices = match engine.get_model() {
        Ok(model) => model
            .get_supported_speakers()
            .unwrap_or_default()
            .into_iter()
            .map(|name| VoiceProfile {
                id: name.clone(),
                name,
                voice_type: "builtin".to_string(),
                language: vec!["zh".to_string(), "en".to_string()],
                preview_audio: None,
                description: Some("Qwen3 TTS 内置音色".to_string()),
            })
            .collect(),
        Err(_) => {
            // 模型未加载时返回默认列表
            vec![
                VoiceProfile {
                    id: "female_01".to_string(),
                    name: "温柔女声".to_string(),
                    voice_type: "builtin".to_string(),
                    language: vec!["zh".to_string()],
                    preview_audio: None,
                    description: Some("适合客服、旁白和引导音。".to_string()),
                },
                VoiceProfile {
                    id: "female_02".to_string(),
                    name: "明亮女声".to_string(),
                    voice_type: "builtin".to_string(),
                    language: vec!["zh".to_string()],
                    preview_audio: None,
                    description: Some("适合短视频和内容播报。".to_string()),
                },
                VoiceProfile {
                    id: "male_01".to_string(),
                    name: "沉稳男声".to_string(),
                    voice_type: "builtin".to_string(),
                    language: vec!["zh".to_string()],
                    preview_audio: None,
                    description: Some("适合解说和资讯播报。".to_string()),
                },
                VoiceProfile {
                    id: "male_02".to_string(),
                    name: "清晰男声".to_string(),
                    voice_type: "builtin".to_string(),
                    language: vec!["zh".to_string()],
                    preview_audio: None,
                    description: Some("适合教程和产品介绍。".to_string()),
                },
                VoiceProfile {
                    id: "narrator_01".to_string(),
                    name: "中性旁白".to_string(),
                    voice_type: "builtin".to_string(),
                    language: vec!["zh".to_string(), "en".to_string()],
                    preview_audio: None,
                    description: Some("适合故事和说明文案。".to_string()),
                },
            ]
        }
    };

    // Custom voices from profiles directory
    let custom_voices = load_custom_voice_profiles(&state.directories.voices);

    VoicesPayload {
        builtin_voices,
        custom_voices,
    }
}

#[tauri::command]
pub async fn generate_speech(
    payload: GenerateSpeechPayload,
    state: State<'_, AppState>,
) -> GenerateSpeechResult {
    let task_id = format!("tts-{}", chrono_now_ms());
    let start = std::time::Instant::now();

    // 尝试一次懒加载（如果模型尚未加载）
    {
        let mut engine = state.tts_engine.lock().unwrap();
        engine.try_load();
    }

    // 获取模型（加载失败则返回错误）
    let mut engine = state.tts_engine.lock().unwrap();
    let model = match engine.get_model() {
        Ok(m) => m,
        Err(e) => {
            return GenerateSpeechResult {
                task_id,
                status: "failed".to_string(),
                audio_path: None,
                error: Some(e),
                duration_ms: start.elapsed().as_millis(),
            };
        }
    };

    let language: Language = payload.language.as_str().into();

    // 判断是内置音色还是自定义克隆音色
    let output_path = state.directories.outputs.join(format!("{task_id}.wav"));

    let result = if is_custom_voice_id(&payload.voice_id, &state.directories.voices) {
        // 声音克隆模式
        let profile_path = state
            .directories
            .voices
            .join("profiles")
            .join(format!("{}.json", payload.voice_id));
        let ref_audio_path = load_ref_audio_path(&profile_path);

        match ref_audio_path {
            Some(ref path) if path.exists() => {
                let ref_audio = qwen3_tts_rs::AudioInput::from(path.to_string_lossy().as_ref());
                model.generate_voice_clone(
                    &payload.text,
                    language,
                    ref_audio,
                    None,  // ref_text — 在 clone_voice 时可以要求用户提供
                    true,  // x_vector_only_mode — 仅使用说话人嵌入，不需要 ICL
                    Some(GenerationParams::new().non_streaming_mode(true)),
                )
            }
            _ => {
                // 克隆音频文件不存在，回退到默认说话人
                model.generate_custom_voice(
                    &payload.text,
                    Speaker::new("female_01"),
                    language,
                    None,
                    Some(GenerationParams::new().non_streaming_mode(true)),
                )
            }
        }
    } else {
        // 内置音色模式
        let speaker = Speaker::new(&payload.voice_id);
        model.generate_custom_voice(
            &payload.text,
            speaker,
            language,
            None,
            Some(GenerationParams::new().non_streaming_mode(true)),
        )
    };

    match result {
        Ok(output) => {
            let waveform = match output.waveform() {
                Some(w) => w,
                None => {
                    return GenerateSpeechResult {
                        task_id,
                        status: "failed".to_string(),
                        audio_path: None,
                        error: Some("生成结果为空".to_string()),
                        duration_ms: start.elapsed().as_millis(),
                    };
                }
            };

            if let Err(e) = qwen3_tts_rs::audio::write_wav_file(
                &output_path,
                waveform,
                output.sample_rate,
            ) {
                return GenerateSpeechResult {
                    task_id,
                    status: "failed".to_string(),
                    audio_path: None,
                    error: Some(format!("写入音频文件失败: {e}")),
                    duration_ms: start.elapsed().as_millis(),
                };
            }

            GenerateSpeechResult {
                task_id,
                status: "success".to_string(),
                audio_path: Some(output_path.to_string_lossy().to_string()),
                error: None,
                duration_ms: start.elapsed().as_millis(),
            }
        }
        Err(e) => GenerateSpeechResult {
            task_id,
            status: "failed".to_string(),
            audio_path: None,
            error: Some(format!("合成失败: {e}")),
            duration_ms: start.elapsed().as_millis(),
        },
    }
}

#[tauri::command]
pub async fn clone_voice(
    payload: CloneVoicePayload,
    state: State<'_, AppState>,
) -> CloneVoiceResult {
    let profile_dir = state.directories.voices.join("profiles");
    std::fs::create_dir_all(&profile_dir).ok();

    let voice_profile_id = format!("voice-user-{}", chrono_now_ms());
    let src = PathBuf::from(&payload.audio_path);
    let ext = src
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_else(|| ".wav".to_string());
    let target_audio = profile_dir.join(format!("{voice_profile_id}{ext}"));

    // Copy reference audio to profiles dir
    if let Err(e) = std::fs::copy(&src, &target_audio) {
        return CloneVoiceResult {
            voice_profile_id,
            status: format!("failed: {e}"),
        };
    }

    // Save profile metadata
    let metadata = serde_json::json!({
        "id": voice_profile_id,
        "name": payload.name,
        "type": "custom",
        "language": [payload.language],
        "description": format!("参考音频: {}", src.file_name().unwrap_or_default().to_string_lossy()),
        "preview_audio": target_audio.to_string_lossy(),
        "local_audio_path": target_audio.to_string_lossy(),
    });

    let meta_path = profile_dir.join(format!("{voice_profile_id}.json"));
    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&metadata).unwrap_or_default(),
    )
    .ok();

    CloneVoiceResult {
        voice_profile_id,
        status: "success".to_string(),
    }
}

#[tauri::command]
pub async fn delete_voice_profile(
    voice_profile_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile_dir = state.directories.voices.join("profiles");
    let meta_path = profile_dir.join(format!("{voice_profile_id}.json"));

    if !meta_path.exists() {
        return Err("Profile 不存在".to_string());
    }

    // Read metadata to find audio file
    let content = std::fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
    let metadata: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;

    // Delete the reference audio file
    if let Some(audio_path) = metadata.get("local_audio_path").and_then(|v| v.as_str()) {
        let audio_file = std::path::Path::new(audio_path);
        if audio_file.exists() {
            std::fs::remove_file(audio_file).ok();
        }
    }

    std::fs::remove_file(&meta_path).map_err(|e| e.to_string())?;

    Ok(())
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn chrono_now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn load_custom_voice_profiles(voices_dir: &std::path::Path) -> Vec<VoiceProfile> {
    let profile_dir = voices_dir.join("profiles");
    if !profile_dir.exists() {
        return vec![];
    }

    let mut profiles = vec![];
    if let Ok(entries) = std::fs::read_dir(&profile_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                    profiles.push(VoiceProfile {
                        id: meta
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        name: meta
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        voice_type: "custom".to_string(),
                        language: meta
                            .get("language")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        preview_audio: meta
                            .get("preview_audio")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        description: meta
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
        }
    }
    profiles
}

fn is_custom_voice_id(voice_id: &str, voices_dir: &std::path::Path) -> bool {
    let profile_path = voices_dir
        .join("profiles")
        .join(format!("{voice_id}.json"));
    profile_path.exists()
}

fn load_ref_audio_path(profile_path: &std::path::Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(profile_path).ok()?;
    let meta: serde_json::Value = serde_json::from_str(&content).ok()?;
    meta.get("local_audio_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
}
