use anyhow::Result;
use reqwest::Client;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

use crate::file_manager::AppDirectories;
use crate::license::LicenseManager;

/// Wrapper around the Qwen3 TTS model with lazy initialization.
pub struct TtsEngine {
    model: Option<qwen3_tts_rs::Qwen3TTSModel>,
    model_path: PathBuf,
    load_error: Option<String>,
}

impl TtsEngine {
    pub fn new(model_path: PathBuf) -> Self {
        Self {
            model: None,
            model_path,
            load_error: None,
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.model.is_some()
    }

    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    /// Lazily load the model on first call.
    pub fn get_model(&mut self) -> Result<&qwen3_tts_rs::Qwen3TTSModel, String> {
        if let Some(ref model) = self.model {
            return Ok(model);
        }
        if let Some(ref err) = self.load_error {
            return Err(err.clone());
        }

        let model_path_str = self.model_path.to_string_lossy().to_string();

        // 自动检测 GPU：有 CUDA 就用 GPU，没有就用 CPU
        let device = tch::Device::cuda_if_available();
        match device {
            tch::Device::Cuda(_) => eprintln!("检测到 CUDA GPU，使用 GPU 推理"),
            _ => eprintln!("未检测到 CUDA GPU，使用 CPU 推理"),
        };

        match qwen3_tts_rs::Qwen3TTSModel::from_pretrained_with_device(&model_path_str, device) {
            Ok(model) => {
                self.model = Some(model);
                Ok(self.model.as_ref().unwrap())
            }
            Err(e) => {
                let msg = format!("模型加载失败: {e}");
                self.load_error = Some(msg.clone());
                Err(msg)
            }
        }
    }

    /// Try to load the model synchronously (called from background).
    pub fn try_load(&mut self) {
        let _ = self.get_model();
    }
}

pub struct AppState {
    pub directories: AppDirectories,
    pub client: Client,
    pub tts_engine: Mutex<TtsEngine>,
    pub license: Mutex<LicenseManager>,
}

impl AppState {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("app-data"));
        let directories = AppDirectories::new(app_data_dir.clone())?;

        let license = Mutex::new(LicenseManager::new(&app_data_dir));

        // Model directory: app data dir / models
        // Users can place Qwen3-TTS model files here
        let model_path = directories.models.clone();

        let tts_engine = Mutex::new(TtsEngine::new(model_path));

        Ok(Self {
            directories,
            client: Client::new(),
            tts_engine,
            license,
        })
    }
}
