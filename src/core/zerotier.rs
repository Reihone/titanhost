use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroTierStatus {
    pub address: String,
    pub online: bool,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroTierNetwork {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "type", default)]
    pub network_type: String,
    #[serde(rename = "assignedAddresses", default)]
    pub assigned_addresses: Vec<String>,
    #[serde(rename = "portDeviceName", default)]
    pub port_device_name: String,
}

/// Helper function to build a reqwest client with a timeout
fn make_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .unwrap_or_default()
}

/// Tries to read the ZeroTier authtoken directly from standard Linux path
pub fn read_authtoken_direct() -> Option<String> {
    std::fs::read_to_string("/var/lib/zerotier-one/authtoken.secret")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() == 24)
}

/// Tries to read the cached ZeroTier authtoken from application config directory
pub fn read_authtoken_cached() -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let cached_path = format!("{}/.config/titanhost/zerotier_token.txt", home);
    std::fs::read_to_string(&cached_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() == 24)
}

/// Saves the ZeroTier authtoken to cache
pub fn save_authtoken_cache(token: &str) -> Result<(), std::io::Error> {
    let home = std::env::var("HOME").unwrap_or_default();
    let dir_path = format!("{}/.config/titanhost", home);
    std::fs::create_dir_all(&dir_path)?;
    let cached_path = format!("{}/zerotier_token.txt", dir_path);
    std::fs::write(&cached_path, token)?;
    Ok(())
}

/// Requests the authtoken via PolicyKit (pkexec)
pub fn fetch_authtoken_via_pkexec() -> Result<String, String> {
    let output = Command::new("pkexec")
        .arg("cat")
        .arg("/var/lib/zerotier-one/authtoken.secret")
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if token.len() == 24 {
                    if let Err(e) = save_authtoken_cache(&token) {
                        eprintln!("Не удалось кэшировать токен ZeroTier: {}", e);
                    }
                    Ok(token)
                } else {
                    Err(format!("Некорректная длина токена: {}", token))
                }
            } else {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                Err(format!(
                    "Ошибка аутентификации (код {}): {}",
                    out.status.code().unwrap_or(-1),
                    err
                ))
            }
        }
        Err(e) => Err(format!("Не удалось запустить pkexec: {}", e)),
    }
}

/// Fetches status from ZeroTier daemon API
pub async fn get_status(token: &str) -> Result<ZeroTierStatus, String> {
    let client = make_client();
    let res = client
        .get("http://127.0.0.1:9993/status")
        .header("X-ZT1-Auth", token)
        .send()
        .await
        .map_err(|e| format!("Не удалось подключиться к API ZeroTier (порт 9993): {}", e))?;

    if res.status().is_success() {
        res.json::<ZeroTierStatus>()
            .await
            .map_err(|e| format!("Не удалось обработать ответ статуса: {}", e))
    } else {
        Err(format!("Ошибка API: код {}", res.status()))
    }
}

/// Fetches networks list from ZeroTier daemon API
pub async fn get_networks(token: &str) -> Result<Vec<ZeroTierNetwork>, String> {
    let client = make_client();
    let res = client
        .get("http://127.0.0.1:9993/network")
        .header("X-ZT1-Auth", token)
        .send()
        .await
        .map_err(|e| format!("Не удалось подключиться к API ZeroTier (порт 9993): {}", e))?;

    if res.status().is_success() {
        res.json::<Vec<ZeroTierNetwork>>()
            .await
            .map_err(|e| format!("Не удалось обработать список сетей: {}", e))
    } else {
        Err(format!("Ошибка API: код {}", res.status()))
    }
}

/// Joins a network via ZeroTier daemon API
pub async fn join_network(token: &str, network_id: &str) -> Result<(), String> {
    let client = make_client();
    let url = format!("http://127.0.0.1:9993/network/{}", network_id);
    let res = client
        .post(&url)
        .header("X-ZT1-Auth", token)
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| format!("Не удалось отправить запрос на подключение: {}", e))?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!("Ошибка подключения к сети. Код: {}", res.status()))
    }
}

/// Leaves a network via ZeroTier daemon API
pub async fn leave_network(token: &str, network_id: &str) -> Result<(), String> {
    let client = make_client();
    let url = format!("http://127.0.0.1:9993/network/{}", network_id);
    let res = client
        .delete(&url)
        .header("X-ZT1-Auth", token)
        .send()
        .await
        .map_err(|e| format!("Не удалось отправить запрос на отключение: {}", e))?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!("Ошибка отключения от сети. Код: {}", res.status()))
    }
}
