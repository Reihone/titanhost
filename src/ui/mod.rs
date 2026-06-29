pub mod tabs;

use crate::core::config_manager::{load_profiles_data, save_profiles_data};
use crate::core::downloader::{download_server_core, sync_mods, sync_plugins};
use crate::core::SERVER_DIR;
use crate::models::mojang::MinecraftPingResponse;
use crate::models::{
    ActiveTab, ConsoleFilter, LauncherConfig, ModConfig, ModdingSubTab, ProfilesData,
    ServerCoreType, ServerStatus,
};
use eframe::egui;
use std::path::Path;
use std::sync::{Arc, Mutex};
use sysinfo::System;

fn is_world_event(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains('<') && lower.contains('>')
        || lower.contains("joined the game")
        || lower.contains("left the game")
        || lower.contains("присоединился к игре")
        || lower.contains("вышел из игры")
        || lower.contains("issued server command")
        || lower.contains("выполнил команду")
        || lower.contains("[server]")
        || lower.contains("[сервер]")
        || lower.contains("has made the advancement")
        || lower.contains("получил достижение")
}

fn draw_history_graph(ui: &mut egui::Ui, history: &[f32], max_val: f32, color: egui::Color32) {
    let desired_size = egui::vec2(ui.available_width(), 60.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Draw background
        painter.rect_filled(
            rect,
            4.0,
            egui::Color32::from_rgba_unmultiplied(25, 25, 25, 255),
        );

        // Draw border
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_unmultiplied(100, 100, 100, 50),
            ),
        );

        if history.len() >= 2 {
            let width = rect.width();
            let height = rect.height();
            let points_count = history.len();

            // Draw grid lines
            for i in 1..4 {
                let y = rect.top() + height * (i as f32 / 4.0);
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    egui::Stroke::new(
                        0.5,
                        egui::Color32::from_rgba_unmultiplied(200, 200, 200, 20),
                    ),
                );
            }

            let mut line_points = Vec::new();
            for (idx, &val) in history.iter().enumerate() {
                let x = rect.left() + (idx as f32 / (points_count - 1) as f32) * width;
                let val_clamped = val.clamp(0.0, max_val);
                let norm = if max_val > 0.0 {
                    val_clamped / max_val
                } else {
                    0.0
                };
                let y = rect.bottom() - norm * (height - 8.0) - 4.0;
                line_points.push(egui::pos2(x, y));
            }

            // Draw filled area under the line
            let mut fill_points = line_points.clone();
            fill_points.push(egui::pos2(rect.right(), rect.bottom() - 1.0));
            fill_points.push(egui::pos2(rect.left(), rect.bottom() - 1.0));
            let fill_color =
                egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 30);
            painter.add(egui::Shape::convex_polygon(
                fill_points,
                fill_color,
                egui::Stroke::NONE,
            ));

            // Draw line segments
            for pair in line_points.windows(2) {
                painter.line_segment([pair[0], pair[1]], egui::Stroke::new(1.5, color));
            }
        } else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Сбор данных нагрузки...",
                egui::FontId::proportional(11.0),
                egui::Color32::GRAY,
            );
        }
    }
}

const MC_COMMANDS: &[&str] = &[
    "help",
    "op",
    "deop",
    "stop",
    "restart",
    "kick",
    "ban",
    "ban-ip",
    "pardon",
    "pardon-ip",
    "list",
    "say",
    "tellraw",
    "whitelist",
    "difficulty",
    "gamemode",
    "gamerule",
    "give",
    "clear",
    "xp",
    "time",
    "weather",
    "teleport",
    "tp",
    "save-all",
    "save-off",
    "save-on",
    "seed",
    "setblock",
    "spawnpoint",
    "summon",
    "effect",
    "enchant",
    "fill",
    "locate",
    "playsound",
    "scoreboard",
    "setworldspawn",
    "spreadplayers",
    "stopsound",
    "title",
    "trigger",
    "reload",
    "function",
    "advancement",
    "recipe",
    "tag",
    "team",
    "bossbar",
    "clone",
    "data",
    "datapack",
    "debug",
    "defaultgamemode",
    "execute",
    "forceload",
    "loot",
    "msg",
    "say",
    "tell",
    "w",
    "publish",
    "spectate",
    "teammsg",
    "tm",
];

fn format_server_command(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // If starts with '/', strip it and send
    if trimmed.starts_with('/') {
        return trimmed[1..].trim().to_string();
    }

    // Get the first word in lowercase
    let first_word = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();

    // If it is a known Minecraft command, send it as-is
    if MC_COMMANDS.contains(&first_word.as_str()) {
        trimmed.to_string()
    } else {
        // Otherwise, wrap in "say" command to post to game chat as Server
        format!("say {}", trimmed)
    }
}

pub struct LauncherApp {
    pub profiles_data: ProfilesData,
    pub selected_profile: String,
    pub config: LauncherConfig,
    pub active_tab: ActiveTab,
    pub modding_sub_tab: ModdingSubTab,

    pub total_system_ram: u32,
    pub status_message: String,

    // UI input fields
    pub new_mod_url: String,
    pub new_plugin_url: String,
    pub new_profile_name: String,

    // UI visibility controls
    pub show_new_profile_input: bool,

    // Channel log messaging
    pub log_messages: Vec<String>,
    pub game_log_messages: Vec<String>,
    pub last_progress_log_time: Option<std::time::Instant>,
    pub stdin_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    pub command_input: String,
    pub tx: std::sync::mpsc::Sender<String>,
    pub rx: std::sync::mpsc::Receiver<String>,

    // Download states
    pub is_downloading: bool,
    pub pending_launch: bool,
    pub is_downloading_java: bool,
    pub is_downloading_loader: bool,
    pub is_downloading_libs: bool,
    pub is_downloading_version: bool,
    pub is_downloading_plugins: bool,
    pub is_downloading_server_core: bool,

    // Active process
    pub active_process: Option<tokio::process::Child>,

    // Server status cache and connection address
    pub server_status: Option<ServerStatus>,
    pub server_address: Arc<Mutex<(String, u16)>>,

    // Hash cache for optimization
    pub hash_cache: std::collections::HashMap<String, (u64, String)>,

    // Restart logic
    pub restart_delay_mins: u32,
    pub restart_at: Option<std::time::Instant>,
    pub pending_restart: bool,
    pub last_restart_notify_sec: Option<u64>,
    pub console_filter: ConsoleFilter,
    pub sys: sysinfo::System,
    pub server_cpu_usage: f32,
    pub server_ram_usage: f32,
    pub last_stats_refresh: Option<std::time::Instant>,
    pub cpu_history: Vec<f32>,
    pub ram_history: Vec<f32>,
    pub backups_list: Vec<crate::core::backup::BackupInfo>,
    pub last_launch_time: Option<std::time::Instant>,
    pub consecutive_crashes: u32,

    pub zerotier_token: Option<String>,
    pub zerotier_status: Option<Result<crate::core::zerotier::ZeroTierStatus, String>>,
    pub zerotier_networks: Option<Result<Vec<crate::core::zerotier::ZeroTierNetwork>, String>>,
    pub zerotier_join_input: String,
    pub is_connecting_zerotier: bool,
}

/// Format MOTD string from parsed Minecraft JSON ping response
pub fn get_motd_text(desc: &serde_json::Value) -> String {
    if let Some(s) = desc.as_str() {
        return s.to_string();
    }
    if let Some(text) = desc.get("text").and_then(|v| v.as_str()) {
        let mut full = text.to_string();
        if let Some(extra) = desc.get("extra").and_then(|v| v.as_array()) {
            for item in extra {
                if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                    full.push_str(t);
                }
            }
        }
        return full;
    }
    "Minecraft Server".to_string()
}

impl LauncherApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut sys = System::new_all();
        sys.refresh_memory();
        let total_ram = (sys.total_memory() / 1024 / 1024 / 1024) as u32;

        let (tx, rx) = std::sync::mpsc::channel();

        // Load profiles configuration
        let (profiles_data, startup_log) = load_profiles_data(total_ram);
        let selected_profile = profiles_data.selected_profile.clone();
        let config = profiles_data
            .profiles
            .get(&selected_profile)
            .cloned()
            .unwrap_or_default();

        let server_address = Arc::new(Mutex::new((config.server_ip.clone(), config.server_port)));

        let tx_status = tx.clone();
        let server_address_clone = server_address.clone();
        let ctx_status = _cc.egui_ctx.clone();

        // Spawn async background server pinger task using tokio
        tokio::spawn(async move {
            loop {
                let (ip, port) = {
                    let lock = server_address_clone.lock().unwrap();
                    lock.clone()
                };

                let start = std::time::Instant::now();
                match crate::core::pinger::ping_server_async(&ip, port).await {
                    Ok(json_str) => {
                        let ping = start.elapsed().as_millis();
                        if let Ok(resp) = serde_json::from_str::<MinecraftPingResponse>(&json_str) {
                            let players_online =
                                resp.players.as_ref().map(|p| p.online).unwrap_or(0);
                            let players_max = resp.players.as_ref().map(|p| p.max).unwrap_or(0);
                            let motd = get_motd_text(&resp.description);
                            let _ = tx_status.send(format!(
                                "SERVER_ONLINE|{}|{}|{}|{}",
                                ping, players_online, players_max, motd
                            ));
                        } else {
                            let _ = tx_status.send("SERVER_OFFLINE".to_string());
                        }
                    }
                    Err(_) => {
                        let _ = tx_status.send("SERVER_OFFLINE".to_string());
                    }
                }
                ctx_status.request_repaint();
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        });

        let zerotier_token = crate::core::zerotier::read_authtoken_direct()
            .or(crate::core::zerotier::read_authtoken_cached());

        let tx_zt = tx.clone();
        let ctx_zt = _cc.egui_ctx.clone();
        tokio::spawn(async move {
            loop {
                let _ = tx_zt.send("ZEROTIER_DISABLED".to_string());
                ctx_zt.request_repaint();
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        });

        let mut app = Self {
            profiles_data,
            selected_profile,
            config,
            active_tab: ActiveTab::Launch,
            modding_sub_tab: ModdingSubTab::CoresAndVersions,
            total_system_ram: total_ram,
            status_message: "Готов к запуску".to_string(),
            new_mod_url: "".to_string(),
            new_plugin_url: "".to_string(),
            new_profile_name: "".to_string(),
            show_new_profile_input: false,
            log_messages: vec![
                format!("💻 Физическая память системы: {} ГБ", total_ram),
                startup_log,
            ],
            game_log_messages: Vec::new(),
            tx,
            rx,
            is_downloading: false,
            pending_launch: false,
            is_downloading_java: false,
            is_downloading_loader: false,
            is_downloading_libs: false,
            is_downloading_version: false,
            is_downloading_plugins: false,
            is_downloading_server_core: false,
            active_process: None,
            server_status: None,
            server_address,
            hash_cache: std::collections::HashMap::new(),
            restart_delay_mins: 0,
            restart_at: None,
            pending_restart: false,
            stdin_tx: None,
            command_input: String::new(),
            last_progress_log_time: None,
            last_restart_notify_sec: None,
            console_filter: ConsoleFilter::All,
            sys,
            server_cpu_usage: 0.0,
            server_ram_usage: 0.0,
            last_stats_refresh: None,
            cpu_history: Vec::new(),
            ram_history: Vec::new(),
            backups_list: Vec::new(),
            last_launch_time: None,
            consecutive_crashes: 0,
            zerotier_token,
            zerotier_status: None,
            zerotier_networks: None,
            zerotier_join_input: String::new(),
            is_connecting_zerotier: false,
        };
        app.backups_list = crate::core::backup::list_backups(crate::core::SERVER_DIR);
        crate::ui::tabs::modding::scan_local_mods(&mut app);
        crate::ui::tabs::modding::scan_local_plugins(&mut app);
        app
    }

    pub fn add_system_log(&mut self, log_entry: String) {
        let is_progress = log_entry.contains('%')
            && (log_entry.contains("Preparing spawn area")
                || log_entry.contains("Загрузка")
                || log_entry.contains("Скачивание")
                || log_entry.contains("библиотеки")
                || log_entry.contains("Progress"));

        if is_progress {
            if let Some(last_line) = self.log_messages.last_mut() {
                if last_line.contains('%')
                    && (last_line.contains("Preparing spawn area")
                        || last_line.contains("Загрузка")
                        || last_line.contains("Скачивание")
                        || last_line.contains("библиотеки")
                        || last_line.contains("Progress"))
                {
                    let now = std::time::Instant::now();
                    let time_elapsed = self
                        .last_progress_log_time
                        .map(|t| now.duration_since(t).as_secs() >= 5)
                        .unwrap_or(true);
                    let is_completed = log_entry.contains("100%")
                        || log_entry.contains("завершено")
                        || log_entry.contains("done")
                        || log_entry.contains("Успешно");

                    if time_elapsed || is_completed {
                        *last_line = log_entry;
                        self.last_progress_log_time = Some(now);
                    }
                    return;
                }
            }
        }

        self.log_messages.push(log_entry);
        if is_progress {
            self.last_progress_log_time = Some(std::time::Instant::now());
        }
        if self.log_messages.len() > 1000 {
            self.log_messages.remove(0);
        }
    }

    pub fn save_config(&mut self) {
        self.profiles_data
            .profiles
            .insert(self.selected_profile.clone(), self.config.clone());
        self.profiles_data.selected_profile = self.selected_profile.clone();

        match save_profiles_data(&self.profiles_data) {
            Ok(_) => {
                self.status_message = "Профиль сохранен!".to_string();
            }
            Err(e) => {
                self.status_message = "Ошибка сохранения!".to_string();
                self.add_system_log(format!(
                    "[СИСТЕМА] Не удалось сохранить profiles.json: {}",
                    e
                ));
            }
        }
    }

    pub fn apply_auto_ram_preset(&mut self) {
        if self.total_system_ram <= 4 {
            self.config.max_ram_gb = 2.min(self.total_system_ram).max(1);
        } else {
            self.config.max_ram_gb = self.total_system_ram.saturating_sub(3).max(2);
        }
        self.status_message = format!("Умный пресет: {} ГБ", self.config.max_ram_gb);
        self.log_messages.push(format!(
            "[ОЗУ] Установлено оптимальное выделение ОЗУ: {} ГБ",
            self.config.max_ram_gb
        ));
    }

    pub fn launch_game(&mut self, ctx: egui::Context) {
        self.status_message = "Запуск сервера...".to_string();
        self.log_messages
            .push("[ЗАПУСК] Проверка файлов сервера...".to_string());

        let server_jar = format!("{}/server.jar", SERVER_DIR);
        if !Path::new(&server_jar).exists() {
            let filename = match self.config.server_core_type {
                ServerCoreType::Vanilla => {
                    format!("minecraft_server_{}.jar", self.config.minecraft_version)
                }
                ServerCoreType::Paper => format!("paper-{}.jar", self.config.minecraft_version),
                ServerCoreType::Forge => {
                    format!("forge-server-{}.jar", self.config.minecraft_version)
                }
                ServerCoreType::Fabric => {
                    format!("fabric-server-{}.jar", self.config.minecraft_version)
                }
                ServerCoreType::NeoForge => {
                    format!("neoforge-server-{}.jar", self.config.minecraft_version)
                }
            };

            self.log_messages.push(format!(
                "[ЗАПУСК] Отсутствует ядро сервера. Автоматическое скачивание {}...",
                filename
            ));
            self.is_downloading_server_core = true;
            self.pending_launch = true;
            let ctx_clone = ctx.clone();
            download_server_core(
                ctx_clone,
                self.tx.clone(),
                self.config.server_core_type,
                self.config.minecraft_version.clone(),
                self.config.server_core_version.clone(),
                filename.clone(),
                format!("{:?} Server Core", self.config.server_core_type),
            );
            return;
        }

        // Generate configurations automatically
        if let Err(e) = crate::core::process::generate_server_files(&self.config) {
            self.log_messages.push(format!(
                "[ПРЕДУПРЕЖДЕНИЕ] Не удалось сгенерировать конфигурации сервера: {}",
                e
            ));
        }

        match crate::core::process::launch_game_process(&self.config, self.tx.clone()) {
            Ok((child, stdin_tx)) => {
                let pid = child.id().unwrap_or(0);
                self.log_messages
                    .push(format!("[УСПЕХ] Сервер запущен! PID: {}", pid));
                self.status_message = "Сервер запущен".to_string();
                self.active_process = Some(child);
                self.stdin_tx = Some(stdin_tx);
                self.last_launch_time = Some(std::time::Instant::now());
            }
            Err(e) => {
                self.log_messages
                    .push(format!("[ОШИБКА] Ошибка старта процесса сервера: {}", e));
                self.status_message = "Ошибка старта процесса Java".to_string();
            }
        }
    }

    pub fn sync_mods_task(&mut self, ctx: egui::Context, launch_after: bool) {
        if self.is_downloading {
            return;
        }
        self.is_downloading = true;
        self.pending_launch = launch_after;
        self.save_config();

        sync_mods(ctx, self.tx.clone(), self.config.mods.clone(), launch_after);
    }

    pub fn sync_plugins_task(&mut self, ctx: egui::Context) {
        if self.is_downloading_plugins {
            return;
        }
        self.is_downloading_plugins = true;
        self.save_config();

        sync_plugins(ctx, self.tx.clone(), self.config.plugins.clone());
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        // Fetch process stats if running
        let mut stats_updated = false;
        if let Some(ref child) = self.active_process {
            if let Some(pid) = child.id() {
                let now = std::time::Instant::now();
                let should_refresh = self
                    .last_stats_refresh
                    .map(|last| now.duration_since(last).as_secs_f32() >= 1.0)
                    .unwrap_or(true);

                if should_refresh {
                    let sys_pid = sysinfo::Pid::from(pid as usize);
                    self.sys.refresh_process(sys_pid);

                    if let Some(proc) = self.sys.process(sys_pid) {
                        let cpus = self.sys.cpus().len().max(1) as f32;
                        let current_cpu = proc.cpu_usage() / cpus;
                        let current_ram = proc.memory() as f32 / 1024.0 / 1024.0 / 1024.0;

                        self.server_cpu_usage = current_cpu;
                        self.server_ram_usage = current_ram;

                        self.cpu_history.push(current_cpu);
                        if self.cpu_history.len() > 60 {
                            self.cpu_history.remove(0);
                        }
                        self.ram_history.push(current_ram);
                        if self.ram_history.len() > 60 {
                            self.ram_history.remove(0);
                        }
                    }
                    self.last_stats_refresh = Some(now);
                    stats_updated = true;
                }
            }
        } else {
            self.server_cpu_usage = 0.0;
            self.server_ram_usage = 0.0;
            if !self.cpu_history.is_empty() {
                self.cpu_history.clear();
                self.ram_history.clear();
            }
        }

        if stats_updated {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        } else if self.active_process.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // Check scheduled restart countdown
        if self.active_process.is_some() {
            if let Some(restart_time) = self.restart_at {
                let now = std::time::Instant::now();
                if now >= restart_time {
                    self.restart_at = None;
                    self.pending_restart = true;
                    if let Some(mut proc) = self.active_process.take() {
                        let _ = proc.start_kill();
                        self.add_system_log(
                            "[СИСТЕМА] Инициирован запланированный перезапуск сервера..."
                                .to_string(),
                        );
                        self.status_message = "Перезапуск сервера...".to_string();
                    }
                } else {
                    let remaining = restart_time.duration_since(now).as_secs();
                    let mins = remaining / 60;
                    let secs = remaining % 60;
                    self.status_message = format!("Перезапуск через {:02}:{:02}", mins, secs);

                    // Periodic notifications to game chat
                    let notify_msg = match remaining {
                        300 => Some("Внимание! Перезапуск сервера через 5 минут!".to_string()),
                        180 => Some("Внимание! Перезапуск сервера через 3 минуты!".to_string()),
                        60 => Some("Внимание! Перезапуск сервера через 1 минуту!".to_string()),
                        30 => Some("Внимание! Перезапуск сервера через 30 секунд!".to_string()),
                        10 => Some("Сервер перезапустится через 10 секунд!".to_string()),
                        5 => Some("Перезапуск через 5 секунд...".to_string()),
                        4 => Some("Перезапуск через 4 секунды...".to_string()),
                        3 => Some("Перезапуск через 3 секунды...".to_string()),
                        2 => Some("Перезапуск через 2 секунды...".to_string()),
                        1 => Some("Перезапуск через 1 секунду!".to_string()),
                        _ => None,
                    };

                    if let Some(msg) = notify_msg {
                        if self.last_restart_notify_sec != Some(remaining) {
                            self.last_restart_notify_sec = Some(remaining);
                            if let Some(ref stdin_tx) = self.stdin_tx {
                                let _ = stdin_tx.send(format!("say {}", msg));
                            }
                        }
                    }

                    // Request repaint to update countdown timer in UI
                    ctx.request_repaint();
                }
            }
        } else {
            // If server is not running, clear scheduled restart
            self.restart_at = None;
        }

        // Process running checks
        if let Some(ref mut child) = self.active_process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let code = status.code().unwrap_or(-1);
                    self.log_messages
                        .push(format!("[СИСТЕМА] Сервер закрылся с кодом: {}.", code));
                    self.status_message = format!("Сервер закрыт (Код {})", code);
                    self.active_process = None;
                    self.stdin_tx = None;

                    if self.pending_restart {
                        self.pending_restart = false;
                        self.launch_game(ctx.clone());
                    } else if self.config.auto_restart_on_crash && code != 0 {
                        self.add_system_log("[АВТОРЕСТАРТ] Обнаружен сбой сервера! Запуск автоматического восстановления...".to_string());
                        self.launch_game(ctx.clone());
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    self.log_messages.push(format!(
                        "[ОШИБКА] Не удалось опросить статус процесса сервера: {}",
                        e
                    ));
                    self.active_process = None;
                    self.stdin_tx = None;

                    if self.pending_restart {
                        self.pending_restart = false;
                        self.launch_game(ctx.clone());
                    } else if self.config.auto_restart_on_crash {
                        self.add_system_log("[АВТОРЕСТАРТ] Обнаружена потеря процесса! Запуск автоматического восстановления...".to_string());
                        self.launch_game(ctx.clone());
                    }
                }
            }
        }

        // Process status queue
        while let Ok(msg) = self.rx.try_recv() {
            if let Some(stripped) = msg.strip_prefix("LOG: ") {
                let log_entry = stripped.to_string();
                if log_entry.contains("[GAME-OUT]") || log_entry.contains("[GAME-ERR]") {
                    let clean_log = log_entry
                        .replace("[GAME-OUT] ", "")
                        .replace("[GAME-ERR] ", "");
                    if is_world_event(&clean_log) {
                        self.game_log_messages.push(clean_log);
                        if self.game_log_messages.len() > 1000 {
                            self.game_log_messages.remove(0);
                        }
                    } else {
                        self.add_system_log(format!("[СЕРВЕР] {}", clean_log));
                    }
                } else {
                    self.add_system_log(log_entry);
                }
            } else if let Some(stripped) = msg.strip_prefix("STATUS: ") {
                self.status_message = stripped.to_string();
            } else if let Some(json) = msg.strip_prefix("UPDATE_MODS:") {
                if let Ok(updated) = serde_json::from_str::<Vec<ModConfig>>(json) {
                    self.config.mods = updated;
                    self.save_config();
                }
            } else if let Some(json) = msg.strip_prefix("UPDATE_PLUGINS:") {
                if let Ok(updated) = serde_json::from_str::<Vec<ModConfig>>(json) {
                    self.config.plugins = updated;
                    self.save_config();
                }
            } else if msg == "FINISH_SUCCESS" {
                self.is_downloading = false;
                self.log_messages
                    .push("[СИСТЕМА] Синхронизация модов завершена.".to_string());
                self.status_message = "Готов".to_string();
                if self.pending_launch {
                    self.pending_launch = false;
                    self.launch_game(ctx.clone());
                }
            } else if msg == "FINISH_ERROR" {
                self.is_downloading = false;
                self.status_message = "Ошибка загрузки модов".to_string();
                if self.pending_launch {
                    self.pending_launch = false;
                    self.log_messages
                        .push("[ЗАПУСК] Отменен из-за ошибок скачивания.".to_string());
                }
            } else if msg == "FINISH_JAVA_SUCCESS" {
                self.is_downloading_java = false;
                self.status_message = "Java установлена".to_string();
                if self.pending_launch {
                    self.launch_game(ctx.clone());
                }
            } else if msg == "FINISH_JAVA_ERROR" {
                self.is_downloading_java = false;
                self.status_message = "Ошибка установки Java".to_string();
                self.pending_launch = false;
            } else if msg == "FINISH_LOADER_SUCCESS" {
                self.is_downloading_loader = false;
                self.status_message = "Ядро установлено".to_string();
                if self.pending_launch {
                    self.launch_game(ctx.clone());
                }
            } else if msg == "FINISH_LOADER_ERROR" {
                self.is_downloading_loader = false;
                self.status_message = "Ошибка загрузки ядра".to_string();
                self.pending_launch = false;
            } else if msg == "FINISH_LIBS_SUCCESS" {
                self.is_downloading_libs = false;
                self.status_message = "Библиотеки установлены".to_string();
                if self.pending_launch {
                    self.launch_game(ctx.clone());
                }
            } else if msg == "FINISH_LIBS_ERROR" {
                self.is_downloading_libs = false;
                self.status_message = "Ошибка загрузки библиотек".to_string();
                self.pending_launch = false;
            } else if msg == "FINISH_VERSION_SUCCESS" {
                self.is_downloading_version = false;
                self.status_message = "Версия игры установлена".to_string();
                if self.pending_launch {
                    self.launch_game(ctx.clone());
                }
            } else if msg == "FINISH_VERSION_ERROR" {
                self.is_downloading_version = false;
                self.status_message = "Ошибка загрузки версии".to_string();
                self.pending_launch = false;
            } else if msg == "FINISH_PLUGINS_SUCCESS" {
                self.is_downloading_plugins = false;
                self.status_message = "Синхронизация плагинов завершена".to_string();
            } else if msg == "FINISH_PLUGINS_ERROR" {
                self.is_downloading_plugins = false;
                self.status_message = "Ошибка синхронизации плагинов".to_string();
            } else if msg == "FINISH_SERVER_CORE_SUCCESS" {
                self.is_downloading_server_core = false;
                self.status_message = "Серверное ядро установлено".to_string();
            } else if msg == "FINISH_SERVER_CORE_ERROR" {
                self.is_downloading_server_core = false;
                self.status_message = "Ошибка установки серверного ядра".to_string();
            } else if let Some(stripped) = msg.strip_prefix("SERVER_ONLINE|") {
                let parts: Vec<&str> = stripped.split('|').collect();
                if parts.len() >= 4 {
                    let ping_ms = parts[0].parse().unwrap_or(0);
                    let players_online = parts[1].parse().unwrap_or(0);
                    let players_max = parts[2].parse().unwrap_or(0);
                    let motd = parts[3..].join("|");
                    self.server_status = Some(ServerStatus {
                        is_online: true,
                        motd,
                        players_online,
                        players_max,
                        ping_ms,
                    });
                }
            } else if msg == "SERVER_OFFLINE" {
                self.server_status = Some(ServerStatus {
                    is_online: false,
                    motd: "".to_string(),
                    players_online: 0,
                    players_max: 0,
                    ping_ms: 0,
                });
            } else if let Some(content) = msg.strip_prefix("ZEROTIER_STATUS|") {
                if content.starts_with("OK|") {
                    if let Ok(status) =
                        serde_json::from_str::<crate::core::zerotier::ZeroTierStatus>(&content[3..])
                    {
                        self.zerotier_status = Some(Ok(status));
                    }
                } else if content.starts_with("ERR|") {
                    self.zerotier_status = Some(Err(content[4..].to_string()));
                }
            } else if let Some(content) = msg.strip_prefix("ZEROTIER_NETWORKS|") {
                if content.starts_with("OK|") {
                    if let Ok(nets) = serde_json::from_str::<
                        Vec<crate::core::zerotier::ZeroTierNetwork>,
                    >(&content[3..])
                    {
                        self.zerotier_networks = Some(Ok(nets));
                    }
                } else if content.starts_with("ERR|") {
                    self.zerotier_networks = Some(Err(content[4..].to_string()));
                }
            } else if msg == "ZEROTIER_NO_TOKEN" {
                self.zerotier_token = None;
                self.zerotier_status = None;
                self.zerotier_networks = None;
            } else if msg == "ZEROTIER_DISABLED" {
                self.zerotier_token = None;
                self.zerotier_status = Some(Err("Функция временно отключена".to_string()));
                self.zerotier_networks = None;
            }
            ctx.request_repaint();
        }

        // Layout Render
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_source("main_scroll_area")
                .show(ui, |ui| {
                    // Header: Server Status info
                    ui.vertical_centered(|ui| {
                        ui.add_space(3.0);
                        ui.heading("🚀 TitanHost Configurator");
                        
                        if let Some(ref status) = self.server_status {
                            if status.is_online {
                                let text = format!("🟢 Сервер онлайн | Игроки: {}/{} | Пинг: {} мс | MOTD: {}", 
                                    status.players_online, status.players_max, status.ping_ms, status.motd);
                                ui.colored_label(egui::Color32::from_rgb(100, 230, 100), text);
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(230, 100, 100), "🔴 Сервер недоступен (Офлайн)");
                            }
                        } else {
                            ui.colored_label(egui::Color32::GRAY, "📡 Поиск игрового сервера...");
                        }

                        if self.active_process.is_some() {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("📊 Процесс:").strong());
                                
                                // CPU bar
                                ui.label(format!("ЦП: {:.1}%", self.server_cpu_usage));
                                let cpu_progress = (self.server_cpu_usage / 100.0).clamp(0.0, 1.0);
                                ui.add(
                                    egui::ProgressBar::new(cpu_progress)
                                        .desired_width(100.0)
                                        .fill(egui::Color32::from_rgb(100, 180, 240))
                                        .text(format!("{:.0}%", self.server_cpu_usage))
                                );

                                ui.add_space(8.0);

                                // RAM bar
                                ui.label(format!("ОЗУ: {:.2} ГБ / {} ГБ", self.server_ram_usage, self.config.max_ram_gb));
                                let ram_progress = (self.server_ram_usage / self.config.max_ram_gb.max(1) as f32).clamp(0.0, 1.0);
                                ui.add(
                                    egui::ProgressBar::new(ram_progress)
                                        .desired_width(100.0)
                                        .fill(egui::Color32::from_rgb(120, 220, 120))
                                        .text(format!("{:.0}%", ram_progress * 100.0))
                                );
                            });

                            ui.add_space(4.0);
                            egui::CollapsingHeader::new(egui::RichText::new("📊 Графики нагрузки CPU и RAM").strong())
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.columns(2, |cols| {
                                        cols[0].vertical(|ui| {
                                            ui.label(egui::RichText::new("Загрузка процессора (CPU)").strong());
                                            draw_history_graph(ui, &self.cpu_history, 100.0, egui::Color32::from_rgb(100, 180, 240));
                                        });
                                        cols[1].vertical(|ui| {
                                            ui.label(egui::RichText::new("Использование памяти (RAM)").strong());
                                            draw_history_graph(ui, &self.ram_history, self.config.max_ram_gb as f32, egui::Color32::from_rgb(120, 220, 120));
                                        });
                                    });
                                });
                        }
                    });


                    ui.add_space(6.0);
                    ui.separator();
                    
                    // Tab Header Buttons
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.active_tab, ActiveTab::Launch, "🎮 Запуск");
                        ui.selectable_value(&mut self.active_tab, ActiveTab::Modding, "🛠️ Моддинг");
                        ui.selectable_value(&mut self.active_tab, ActiveTab::ServerSettings, "📡 Сервер");
                        ui.selectable_value(&mut self.active_tab, ActiveTab::Backups, "💾 Бэкапы");
                    });

                    ui.separator();
                    ui.add_space(4.0);

                    // Tab Body routing
                    match self.active_tab {
                        ActiveTab::Launch => {
                            tabs::launch::draw_launch_tab(self, ui, ctx);
                        }
                        ActiveTab::Modding => {
                            tabs::modding::draw_modding_tab(self, ui, ctx);
                        }
                        ActiveTab::ServerSettings => {
                            tabs::server::draw_server_tab(self, ui);
                        }
                        ActiveTab::Backups => {
                            tabs::backups::draw_backups_tab(self, ui);
                        }
                    }


                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Collapsible System Logs Section
                    egui::CollapsingHeader::new(egui::RichText::new("📟 Системный монитор логов").strong())
                        .default_open(false)
                        .show(ui, |ui| {
                            egui::Resize::default()
                                .id_source("system_log_resize")
                                .default_size([ui.available_width() - 10.0, 80.0])
                                .show(ui, |ui| {
                                    let mut log_text = self.log_messages.join("\n");
                                    let avail_size = ui.available_size();
                                    egui::ScrollArea::vertical()
                                        .id_source("system_log_scroll")
                                        .max_height(avail_size.y)
                                        .max_width(avail_size.x - 16.0)
                                        .stick_to_bottom(true)
                                        .show(ui, |ui| {
                                            ui.add(
                                                egui::TextEdit::multiline(&mut log_text)
                                                    .font(egui::TextStyle::Monospace)
                                                    .desired_width(avail_size.x - 24.0)
                                                    .desired_rows((avail_size.y / 15.0).max(2.0) as usize)
                                            );
                                        });
                                });
                        });

                    ui.add_space(4.0);

                    // Collapsible Game/Server Console Logs Section
                    egui::CollapsingHeader::new(egui::RichText::new("🎮 Консоль сервера (Мир Minecraft)").strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::Resize::default()
                                .id_source("game_log_resize")
                                .default_size([ui.available_width() - 10.0, 180.0])
                                .show(ui, |ui| {
                                    let avail_size = ui.available_size();
                                    
                                    // Log filtering controls
                                    ui.horizontal(|ui| {
                                        ui.small("Фильтр:");
                                        ui.selectable_value(&mut self.console_filter, ConsoleFilter::All, "Все");
                                        ui.selectable_value(&mut self.console_filter, ConsoleFilter::Events, "События");
                                        ui.selectable_value(&mut self.console_filter, ConsoleFilter::Info, "Инфо");
                                        ui.selectable_value(&mut self.console_filter, ConsoleFilter::Warn, "Варнинги");
                                        ui.selectable_value(&mut self.console_filter, ConsoleFilter::Error, "Ошибки");
                                    });
                                    ui.add_space(4.0);

                                    let get_color = |line: &str| {
                                        let lower = line.to_lowercase();
                                        if lower.contains("error") || lower.contains("exception") || lower.contains("err") || lower.contains("fatal") {
                                            egui::Color32::from_rgb(255, 120, 120)
                                        } else if lower.contains("warn") || lower.contains("warning") {
                                            egui::Color32::from_rgb(250, 210, 100)
                                        } else if lower.contains("joined the game") || lower.contains("left the game") || lower.contains("присоединился") || lower.contains("вышел") {
                                            egui::Color32::from_rgb(120, 240, 120)
                                        } else if lower.contains("[ввод]") || lower.contains(">") {
                                            egui::Color32::from_rgb(120, 200, 255)
                                        } else {
                                            egui::Color32::from_rgb(210, 210, 210)
                                        }
                                    };

                                    let filtered_lines: Vec<&String> = self.log_messages.iter().filter(|line| {
                                        match self.console_filter {
                                            ConsoleFilter::All => true,
                                            ConsoleFilter::Info => {
                                                let lower = line.to_lowercase();
                                                !lower.contains("error") && !lower.contains("exception") && !lower.contains("warn") && !lower.contains("warning") && !lower.contains("fatal")
                                            }
                                            ConsoleFilter::Warn => {
                                                let lower = line.to_lowercase();
                                                lower.contains("warn") || lower.contains("warning")
                                            }
                                            ConsoleFilter::Error => {
                                                let lower = line.to_lowercase();
                                                lower.contains("error") || lower.contains("exception") || lower.contains("err") || lower.contains("fatal")
                                            }
                                            ConsoleFilter::Events => {
                                                is_world_event(line)
                                            }
                                        }
                                    }).collect();

                                    egui::ScrollArea::vertical()
                                        .id_source("game_log_scroll")
                                        .max_height((avail_size.y - 70.0).max(20.0))
                                        .max_width(avail_size.x - 16.0)
                                        .stick_to_bottom(true)
                                        .show(ui, |ui| {
                                            for line in filtered_lines {
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(line)
                                                            .font(egui::FontId::monospace(12.0))
                                                            .color(get_color(line))
                                                    )
                                                    .wrap(true)
                                                );
                                            }

                                        });

                                    ui.add_space(4.0);

                                    // Quick command shortcuts
                                    ui.vertical(|ui| {
                                        let is_pre = |v: &str| {
                                            if let Some(minor_str) = v.split('.').nth(1) {
                                                if let Ok(minor) = minor_str.parse::<u32>() {
                                                    return minor < 13;
                                                }
                                            }
                                            false
                                        };
                                        let version = &self.config.minecraft_version;
                                        let pre = is_pre(version);

                                        ui.horizontal(|ui| {
                                            ui.small("🖥️ Сервер:");
                                            let cmds = &[
                                                ("👥 Игроки", "list"),
                                                ("💾 Сохранить мир", "save-all"),
                                                ("⏱️ TPS", "tps"),
                                                ("📊 Отчет лагов", "timings paste"),
                                                ("🔄 Релоад", "reload"),
                                                ("🛑 Стоп", "stop"),
                                            ];
                                            
                                            for &(btn_label, cmd_text) in cmds {
                                                if ui.small_button(btn_label).clicked() {
                                                    let formatted = format_server_command(cmd_text);
                                                    if let Some(ref stdin_tx) = self.stdin_tx {
                                                        let _ = stdin_tx.send(formatted.clone());
                                                        self.add_system_log(format!("[ВВОД] Отправлено: {}", formatted));
                                                    }
                                                }
                                            }
                                        });

                                        ui.horizontal(|ui| {
                                            ui.small("📝 Whitelist:");
                                            if ui.small_button("🟢 Вкл").clicked() {
                                                if let Some(ref stdin_tx) = self.stdin_tx {
                                                    let _ = stdin_tx.send("whitelist on".to_string());
                                                    self.add_system_log("[ВВОД] Отправлено: whitelist on".to_string());
                                                }
                                            }
                                            if ui.small_button("🔴 Выкл").clicked() {
                                                if let Some(ref stdin_tx) = self.stdin_tx {
                                                    let _ = stdin_tx.send("whitelist off".to_string());
                                                    self.add_system_log("[ВВОД] Отправлено: whitelist off".to_string());
                                                }
                                            }
                                            if ui.small_button("🔄 Обновить").clicked() {
                                                if let Some(ref stdin_tx) = self.stdin_tx {
                                                    let _ = stdin_tx.send("whitelist reload".to_string());
                                                    self.add_system_log("[ВВОД] Отправлено: whitelist reload".to_string());
                                                }
                                            }
                                            if ui.small_button("➕ Добавить").clicked() {
                                                self.command_input = "whitelist add <Игрок>".to_string();
                                            }
                                            if ui.small_button("➖ Удалить").clicked() {
                                                self.command_input = "whitelist remove <Игрок>".to_string();
                                            }
                                        });

                                        ui.horizontal(|ui| {
                                            ui.small("⚙️ Настройки:");
                                            if ui.small_button("📦 Сохранять вещи").clicked() {
                                                self.command_input = "gamerule keepInventory <true/false>".to_string();
                                            }
                                            if ui.small_button("🔥 Пожары").clicked() {
                                                self.command_input = "gamerule doFireTick <true/false>".to_string();
                                            }
                                            if ui.small_button("☀️ Цикл времени").clicked() {
                                                self.command_input = "gamerule doDaylightCycle <true/false>".to_string();
                                            }
                                            if ui.small_button("🧟 Спавн мобов").clicked() {
                                                self.command_input = "gamerule doMobSpawning <true/false>".to_string();
                                            }
                                        });

                                        ui.horizontal(|ui| {
                                            ui.small("🧹 Очистка/Мир:");
                                            if ui.small_button("🧹 Очистить дроп (лаг)").clicked() {
                                                if let Some(ref stdin_tx) = self.stdin_tx {
                                                    let _ = stdin_tx.send("kill @e[type=item]".to_string());
                                                    self.add_system_log("[ВВОД] Отправлено: kill @e[type=item]".to_string());
                                                }
                                            }
                                            if ui.small_button("☀️ Ясно").clicked() {
                                                if let Some(ref stdin_tx) = self.stdin_tx {
                                                    let _ = stdin_tx.send("weather clear".to_string());
                                                    self.add_system_log("[ВВОД] Отправлено: weather clear".to_string());
                                                }
                                            }
                                            if ui.small_button("🌧️ Дождь").clicked() {
                                                if let Some(ref stdin_tx) = self.stdin_tx {
                                                    let _ = stdin_tx.send("weather rain".to_string());
                                                    self.add_system_log("[ВВОД] Отправлено: weather rain".to_string());
                                                }
                                            }
                                            if ui.small_button("🚧 Граница мира").clicked() {
                                                self.command_input = "worldborder set <размер>".to_string();
                                            }
                                        });

                                        ui.horizontal(|ui| {
                                            ui.small("🏃 Игрок:");
                                            if ui.small_button("❄️ Заморозить").clicked() {
                                                self.command_input = if pre {
                                                    "effect <Игрок> slowness 99999 255 true".to_string()
                                                } else {
                                                    "effect give <Игрок> minecraft:slowness 99999 255 true".to_string()
                                                };
                                            }
                                            if ui.small_button("🔥 Разморозить").clicked() {
                                                self.command_input = if pre {
                                                    "effect <Игрок> clear".to_string()
                                                } else {
                                                    "effect clear <Игрок> minecraft:slowness".to_string()
                                                };
                                            }
                                        });
                                    });

                                    ui.add_space(4.0);

                                    // Command input row (inside Resize container)
                                    ui.horizontal(|ui| {
                                        let text_edit = egui::TextEdit::singleline(&mut self.command_input)
                                            .hint_text("Введите команду сервера (например, op Player или /say привет)...")
                                            .desired_width(avail_size.x - 110.0);

                                        let response = ui.add(text_edit);

                                        // Check if Enter was pressed inside the text edit
                                        let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                        let send_clicked = ui.button("⚡ Отправить").clicked();

                                        if (enter_pressed || send_clicked) && !self.command_input.trim().is_empty() {
                                            let cmd = self.command_input.trim().to_string();
                                            let formatted_cmd = format_server_command(&cmd);

                                            if let Some(ref stdin_tx) = self.stdin_tx {
                                                if stdin_tx.send(formatted_cmd.clone()).is_ok() {
                                                    self.add_system_log(format!("[ВВОД] Отправлено: {}", formatted_cmd));
                                                } else {
                                                    self.add_system_log("[ОШИБКА] Не удалось отправить команду (канал закрыт)".to_string());
                                                }
                                            } else {
                                                self.add_system_log("[ПРЕДУПРЕЖДЕНИЕ] Невозможно отправить команду: сервер не запущен!".to_string());
                                            }

                                            self.command_input.clear();
                                            response.request_focus();
                                        }
                                    });
                                });
                        });


                    ui.add_space(6.0);

                    // Bottom Buttons Bar: Save config, sync mods, Launch / Stop buttons
                    // Bottom Row 1: Save/Sync on left, Start/Restart on right
                    ui.horizontal(|ui| {
                        if ui.button("📥 Сохранить").clicked() {
                            self.save_config();
                        }

                        if ui.button("🔄 Синхронизировать всё").clicked() {
                            self.sync_mods_task(ctx.clone(), false);
                            self.sync_plugins_task(ctx.clone());
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.active_process.is_some() {
                                // 1. Restart / Cancel Restart button (adds first -> right side in RTL)
                                if self.restart_at.is_some() {
                                    let cancel_btn = ui.add(
                                        egui::Button::new(egui::RichText::new("❌ ОТМЕНИТЬ РЕСТАРТ").strong())
                                            .fill(egui::Color32::from_rgb(180, 80, 80))
                                            .min_size(egui::vec2(150.0, 30.0))
                                    );
                                    if cancel_btn.clicked() {
                                        self.restart_at = None;
                                        self.add_system_log("[СИСТЕМА] Отмена запланированного перезапуска.".to_string());
                                        self.status_message = "Готов".to_string();
                                    }
                                } else {
                                    let restart_btn = ui.add(
                                        egui::Button::new(egui::RichText::new("🔄 РЕСТАРТ").strong())
                                            .fill(egui::Color32::from_rgb(200, 120, 40))
                                            .min_size(egui::vec2(120.0, 30.0))
                                    );
                                    if restart_btn.clicked() {
                                        if self.restart_delay_mins == 0 {
                                            self.pending_restart = true;
                                            if let Some(mut proc) = self.active_process.take() {
                                                let _ = proc.start_kill();
                                                self.add_system_log("[СИСТЕМА] Перезапуск сервера...".to_string());
                                                self.status_message = "Перезапуск сервера...".to_string();
                                            }
                                        } else {
                                            self.restart_at = Some(std::time::Instant::now() + std::time::Duration::from_secs(self.restart_delay_mins as u64 * 60));
                                            self.last_restart_notify_sec = None;
                                            self.add_system_log(format!("[СИСТЕМА] Запланирован перезапуск через {} мин.", self.restart_delay_mins));
                                            
                                            if let Some(ref stdin_tx) = self.stdin_tx {
                                                let _ = stdin_tx.send(format!("say Внимание! Запланирован перезапуск сервера через {} мин.", self.restart_delay_mins));
                                            }
                                        }
                                    }
                                }

                                // 2. Drag value input field (adds second -> left of button in RTL)
                                ui.add(
                                    egui::DragValue::new(&mut self.restart_delay_mins)
                                        .speed(1.0)
                                        .clamp_range(0..=1440)
                                        .suffix(" мин")
                                );

                                // 3. Label (adds third -> left of DragValue in RTL)
                                ui.label("через сколько рестарт:");
                            } else {
                                let launch_btn_text = if self.is_downloading { 
                                    "⏳ СВЕРКА МОДОВ..." 
                                } else if self.is_downloading_plugins {
                                    "⏳ СВЕРКА ПЛАГИНОВ..."
                                } else if self.is_downloading_java {
                                    "⏳ СКАЧИВАНИЕ JAVA..."
                                } else if self.is_downloading_loader {
                                    "⏳ СКАЧИВАНИЕ ЯДРА..."
                                } else if self.is_downloading_libs {
                                    "⏳ СКАЧИВАНИЕ БИБЛИОТЕК..."
                                } else if self.is_downloading_version {
                                    "⏳ СКАЧИВАНИЕ ВЕРСИИ..."
                                } else if self.is_downloading_server_core {
                                    "⏳ СКАЧИВАНИЕ СЕРВЕРА..."
                                } else {
                                    "🚀 ЗАПУСТИТЬ СЕРВЕР"
                                };

                                let is_busy = self.is_downloading 
                                    || self.is_downloading_plugins
                                    || self.is_downloading_java 
                                    || self.is_downloading_loader 
                                    || self.is_downloading_libs 
                                    || self.is_downloading_version 
                                    || self.is_downloading_server_core;

                                let launch_btn = ui.add_enabled(
                                    !is_busy,
                                    egui::Button::new(egui::RichText::new(launch_btn_text).strong())
                                        .fill(egui::Color32::from_rgb(50, 150, 50))
                                        .min_size(egui::vec2(150.0, 30.0))
                                );

                                if launch_btn.clicked() {
                                    self.sync_mods_task(ctx.clone(), true);
                                }

                                if is_busy {
                                    ui.spinner();
                                }
                            }
                        });
                    });

                    ui.add_space(4.0);

                    // Bottom Row 2: Stop button on right
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let is_running = self.active_process.is_some();
                            let stop_btn = ui.add_enabled(
                                is_running,
                                egui::Button::new(egui::RichText::new("🛑 ОСТАНОВИТЬ").strong())
                                    .fill(egui::Color32::from_rgb(180, 50, 50))
                                    .min_size(egui::vec2(150.0, 30.0))
                            );

                            if stop_btn.clicked() {
                                if let Some(mut proc) = self.active_process.take() {
                                    let _ = proc.start_kill();
                                    self.add_system_log("[СИСТЕМА] Сервер принудительно остановлен.".to_string());
                                    self.status_message = "Сервер принудительно остановлен".to_string();
                                }
                            }
                        });
                    });
                });
        });
    }
}
