use crate::core::error::AppError;
use crate::core::SERVER_DIR;
use crate::models::{LauncherConfig, ProfilesData};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Load user profiles, automatically fallback or migrate config profiles format if necessary
pub fn load_profiles_data(total_ram: u32) -> (ProfilesData, String) {
    let _ = fs::create_dir_all(SERVER_DIR);
    let profiles_json_path = Path::new(SERVER_DIR).join("profiles.json");
    let old_profile_json_path = Path::new(SERVER_DIR).join("profile.json");
    let local_profiles_json = Path::new("profiles.json");
    let local_old_profile_json = Path::new("profile.json");

    // Copy or migrate from local directory if exists to isolated server dir
    if local_profiles_json.exists() && !profiles_json_path.exists() {
        let _ = fs::copy(local_profiles_json, &profiles_json_path);
    }
    if local_old_profile_json.exists() && !old_profile_json_path.exists() {
        let _ = fs::copy(local_old_profile_json, &old_profile_json_path);
    }

    if profiles_json_path.exists() {
        if let Ok(file) = File::open(&profiles_json_path) {
            if let Ok(mut data) = serde_json::from_reader::<_, ProfilesData>(file) {
                let mut startup_log =
                    "✅ Успешно загружен список профилей из profiles.json".to_string();
                for (name, config) in data.profiles.iter_mut() {
                    let ram_limit = total_ram.max(16);
                    if config.max_ram_gb > ram_limit {
                        config.max_ram_gb = ram_limit.min(config.max_ram_gb).max(1);
                        startup_log.push_str(&format!(
                            "\n⚠️ Профиль '{}' адаптирован под текущее ОЗУ.",
                            name
                        ));
                    }
                }
                return (data, startup_log);
            }
        }
    }

    // Migration from old format if profile.json exists
    if old_profile_json_path.exists() {
        if let Ok(file) = File::open(&old_profile_json_path) {
            if let Ok(mut config) = serde_json::from_reader::<_, LauncherConfig>(file) {
                let startup_log =
                    "✅ Старый профиль 'profile.json' перенесен в новый формат.".to_string();
                let ram_limit = total_ram.max(16);
                if config.max_ram_gb > ram_limit {
                    config.max_ram_gb = ram_limit.min(config.max_ram_gb).max(1);
                }
                let mut profiles = std::collections::HashMap::new();
                profiles.insert("Forge Pack".to_string(), config);
                let data = ProfilesData {
                    selected_profile: "Forge Pack".to_string(),
                    profiles,
                };
                let _ = File::create(&profiles_json_path).and_then(|mut f| {
                    let json = serde_json::to_string_pretty(&data).unwrap_or_default();
                    f.write_all(json.as_bytes())
                });
                let _ = fs::remove_file(&old_profile_json_path);
                let _ = fs::remove_file(local_old_profile_json);
                return (data, startup_log);
            }
        }
    }

    // Default configuration if no profile file exists
    let mut default_config = LauncherConfig::default();
    default_config.max_ram_gb = if total_ram <= 4 {
        2.min(total_ram).max(1)
    } else {
        total_ram.saturating_sub(3).max(2)
    };

    let mut profiles = std::collections::HashMap::new();
    profiles.insert("Forge Pack".to_string(), default_config);

    let data = ProfilesData {
        selected_profile: "Forge Pack".to_string(),
        profiles,
    };

    let _ = File::create(&profiles_json_path).and_then(|mut f| {
        let json = serde_json::to_string_pretty(&data).unwrap_or_default();
        f.write_all(json.as_bytes())
    });

    (
        data,
        "ℹ️ Создана новая конфигурация профилей profiles.json".to_string(),
    )
}

/// Save profile configurations back to disk
pub fn save_profiles_data(data: &ProfilesData) -> Result<(), AppError> {
    let _ = fs::create_dir_all(SERVER_DIR);
    let path = Path::new(SERVER_DIR).join("profiles.json");
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, data)?;
    Ok(())
}
