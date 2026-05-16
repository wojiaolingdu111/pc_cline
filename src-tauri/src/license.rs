use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const TRIAL_DAYS: u64 = 7;
pub const VERIFY_URL: &str = "https://pc-clinet-navy.vercel.app/api/license/verify";
pub const ACTIVATE_URL: &str = "https://pc-clinet-navy.vercel.app/api/license/activate";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LicenseStatus {
    Trial,
    Active,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub status: LicenseStatus,
    pub trial_days_total: u32,
    pub trial_days_left: i64,
    pub license_key: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LicenseStore {
    trial_start_ms: u64,
    license_key: Option<String>,
    machine_id: String,
}

pub struct LicenseManager {
    store: Option<LicenseStore>,
    path: PathBuf,
}

impl LicenseManager {
    pub fn new(app_data_dir: &PathBuf) -> Self {
        let path = app_data_dir.join("license.json");

        let store = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<LicenseStore>(&s).ok())
        } else {
            None
        };

        if store.is_none() {
            let now = now_ms();
            let store = LicenseStore {
                trial_start_ms: now,
                license_key: None,
                machine_id: format!("{:x}", {
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    use std::hash::Hash;
                    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH).hash(&mut h);
                    use std::hash::Hasher;
                    h.finish()
                }),
            };
            if let Ok(json) = serde_json::to_string_pretty(&store) {
                let _ = std::fs::write(&path, &json);
            }
            return Self {
                store: Some(store),
                path,
            };
        }

        Self { store, path }
    }

    pub fn get_info(&self) -> LicenseInfo {
        let store = match &self.store {
            Some(s) => s,
            None => {
                return LicenseInfo {
                    status: LicenseStatus::Trial,
                    trial_days_total: TRIAL_DAYS as u32,
                    trial_days_left: TRIAL_DAYS as i64,
                    license_key: None,
                    message: format!("试用期还剩 {} 天", TRIAL_DAYS),
                }
            }
        };

        if store.license_key.is_some() {
            return LicenseInfo {
                status: LicenseStatus::Active,
                trial_days_total: TRIAL_DAYS as u32,
                trial_days_left: 0,
                license_key: store.license_key.clone(),
                message: "已激活正版授权".to_string(),
            };
        }

        let elapsed_days = (now_ms().saturating_sub(store.trial_start_ms)) / (1000 * 60 * 60 * 24);
        let days_left = (TRIAL_DAYS as i64).saturating_sub(elapsed_days as i64).max(0);

        if days_left > 0 {
            LicenseInfo {
                status: LicenseStatus::Trial,
                trial_days_total: TRIAL_DAYS as u32,
                trial_days_left: days_left,
                license_key: None,
                message: format!("试用期还剩 {} 天", days_left),
            }
        } else {
            LicenseInfo {
                status: LicenseStatus::Expired,
                trial_days_total: TRIAL_DAYS as u32,
                trial_days_left: 0,
                license_key: None,
                message: "试用已过期，请购买授权激活".to_string(),
            }
        }
    }

    pub fn set_license_key(&mut self, key: String) -> Result<()> {
        let store = self
            .store
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("license store not initialized"))?;
        store.license_key = Some(key);
        let json = serde_json::to_string_pretty(&store)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn machine_id(&self) -> String {
        self.store
            .as_ref()
            .map(|s| s.machine_id.clone())
            .unwrap_or_default()
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
