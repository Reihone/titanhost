use super::enums::{ModLoader, ServerCoreType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration details of a single mod or plugin to download
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ModConfig {
    pub url: String,
    pub filename: String,
    pub allow_update: bool,
    pub sha256: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_online_mode() -> bool {
    false
}
fn default_white_list() -> bool {
    false
}
fn default_difficulty() -> String {
    "normal".to_string()
}
fn default_pvp() -> bool {
    true
}
fn default_spawn_monsters() -> bool {
    true
}
fn default_max_players() -> u32 {
    20
}
fn default_view_distance() -> u32 {
    10
}
fn default_auto_restart() -> bool {
    false
}

fn default_allow_flight() -> bool {
    false
}
fn default_level_seed() -> String {
    "".to_string()
}
fn default_spawn_protection() -> u32 {
    16
}
fn default_hardcore() -> bool {
    false
}
fn default_generate_structures() -> bool {
    true
}
fn default_enable_command_block() -> bool {
    false
}
fn default_gamemode() -> String {
    "survival".to_string()
}

/// Profile configurations of a server/instance pack
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LauncherConfig {
    pub profile_name: String,
    pub minecraft_version: String,
    pub max_ram_gb: u32,
    pub aikars_flags: bool,
    pub mods: Vec<ModConfig>,
    pub server_ip: String,
    pub server_port: u16,
    pub mod_loader: ModLoader,
    pub plugins: Vec<ModConfig>,
    pub server_core_type: ServerCoreType,
    pub server_core_version: String,

    #[serde(default = "default_online_mode")]
    pub online_mode: bool,
    #[serde(default = "default_white_list")]
    pub white_list: bool,
    #[serde(default = "default_difficulty")]
    pub difficulty: String,
    #[serde(default = "default_pvp")]
    pub pvp: bool,
    #[serde(default = "default_spawn_monsters")]
    pub spawn_monsters: bool,
    #[serde(default = "default_max_players")]
    pub max_players: u32,
    #[serde(default = "default_view_distance")]
    pub view_distance: u32,
    #[serde(default = "default_auto_restart")]
    pub auto_restart_on_crash: bool,

    #[serde(default = "default_allow_flight")]
    pub allow_flight: bool,
    #[serde(default = "default_level_seed")]
    pub level_seed: String,
    #[serde(default = "default_spawn_protection")]
    pub spawn_protection: u32,
    #[serde(default = "default_hardcore")]
    pub hardcore: bool,
    #[serde(default = "default_generate_structures")]
    pub generate_structures: bool,
    #[serde(default = "default_enable_command_block")]
    pub enable_command_block: bool,
    #[serde(default = "default_gamemode")]
    pub gamemode: String,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            profile_name: "Forge Server Pack".to_string(),
            minecraft_version: "1.12.2".to_string(),
            max_ram_gb: 4,
            aikars_flags: true,
            mods: Vec::new(),
            server_ip: "127.0.0.1".to_string(),
            server_port: 25565,
            mod_loader: ModLoader::Forge,
            plugins: Vec::new(),
            server_core_type: ServerCoreType::Paper,
            server_core_version: "Последний стабильный".to_string(),
            online_mode: false,
            white_list: false,
            difficulty: "normal".to_string(),
            pvp: true,
            spawn_monsters: true,
            max_players: 20,
            view_distance: 10,
            auto_restart_on_crash: false,
            allow_flight: false,
            level_seed: "".to_string(),
            spawn_protection: 16,
            hardcore: false,
            generate_structures: true,
            enable_command_block: false,
            gamemode: "survival".to_string(),
        }
    }
}

/// Collection of profiles stored in profiles.json
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfilesData {
    pub selected_profile: String,
    pub profiles: HashMap<String, LauncherConfig>,
}
