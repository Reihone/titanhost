use crate::core::error::AppError;
use crate::core::SERVER_DIR;
use crate::models::LauncherConfig;
use std::fs;
use std::io::Write;
use std::path::Path;
use tokio::io::AsyncBufReadExt;

/// Supported Minecraft versions list
pub const MINECRAFT_VERSIONS: &[&str] = &[
    "1.20.4", "1.20.1", "1.19.2", "1.18.2", "1.16.5", "1.12.2", "1.7.10",
];

/// Recommended Java garbage collector configuration parameters (Aikar's Flags)
pub const AIKARS_FLAGS: &[&str] = &[
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:MaxGCPauseMillis=200",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+DisableExplicitGC",
    "-XX:+AlwaysPreTouch",
    "-XX:G1NewSizePercent=30",
    "-XX:G1MaxNewSizePercent=40",
    "-XX:G1ReservePercent=15",
    "-XX:G1HeapRegionSize=32m",
    "-XX:G1MixedGCCountTarget=8",
    "-XX:InitiatingHeapOccupancyPercent=15",
    "-XX:G1MixedGCLiveThresholdPercent=90",
    "-XX:G1RSetUpdatingPauseTimePercent=5",
    "-XX:SurvivorRatio=32",
    "-XX:+PerfDisableSharedMem",
    "-XX:MaxTenuringThreshold=1",
];

/// Get URL check for version
pub fn get_minecraft_version_url(version: &str) -> Option<&'static str> {
    if MINECRAFT_VERSIONS.contains(&version) {
        Some("dynamic")
    } else {
        None
    }
}

/// Choose proper Java JRE version based on targeted Minecraft version rules
pub fn get_java_version_for_minecraft(mc_version: &str) -> u32 {
    if mc_version.starts_with("1.20.5")
        || mc_version.starts_with("1.21")
        || mc_version.starts_with("2")
        || mc_version.starts_with("24w")
        || mc_version.starts_with("25w")
        || mc_version.starts_with("26w")
    {
        21
    } else if mc_version.starts_with("1.16")
        || mc_version.starts_with("1.17")
        || mc_version.starts_with("1.18")
        || mc_version.starts_with("1.19")
        || mc_version.starts_with("1.20")
    {
        17
    } else {
        8
    }
}

/// Check if local matching runtime JRE is downloaded, otherwise default to global "java" command
pub fn get_java_executable(java_ver: u32) -> String {
    let local_path = format!("{}/jre{}/bin/java", SERVER_DIR, java_ver);
    if Path::new(&local_path).exists() {
        local_path
    } else {
        "java".to_string()
    }
}

/// Helper method to collect all Jars in the directory recursively
pub fn collect_jars(dir: &Path, jars: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_jars(&path, jars);
            } else if path.is_file() && path.extension().map(|e| e == "jar").unwrap_or(false) {
                if let Some(path_str) = path.to_str() {
                    jars.push(path_str.to_string());
                }
            }
        }
    }
}

/// Streams stdout/stderr from process to console UI logs list and log files in background
pub fn pipe_stream<R>(
    stream: R,
    tx: std::sync::mpsc::Sender<String>,
    prefix: &'static str,
    log_file_path: String,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stream);
        let mut line = String::new();
        while let Ok(bytes) = reader.read_line(&mut line).await {
            if bytes == 0 {
                break;
            }
            let trimmed = line.trim().to_string();
            if !trimmed.is_empty() {
                if let Ok(mut f) = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_file_path)
                {
                    let _ = writeln!(f, "[{}] {}", prefix, trimmed);
                }
                let _ = tx.send(format!("LOG: [{}] {}", prefix, trimmed));
            }
            line.clear();
        }
    });
}

/// Automate generation of server files: eula.txt, server.properties and run.bat / run.sh
pub fn generate_server_files(config: &LauncherConfig) -> Result<(), std::io::Error> {
    // Ensure base server path exists
    let _ = fs::create_dir_all(SERVER_DIR);

    // 1. Generate eula.txt
    let mut eula = std::fs::File::create(format!("{}/eula.txt", SERVER_DIR))?;
    writeln!(eula, "#By changing the setting below to TRUE you are indicating your agreement to our EULA (https://account.mojang.com/documents/minecraft_eula).")?;
    writeln!(eula, "eula=true")?;

    // 2. Generate server.properties
    let mut props = std::fs::File::create(format!("{}/server.properties", SERVER_DIR))?;
    writeln!(props, "#Minecraft server properties")?;
    writeln!(props, "server-ip={}", config.server_ip)?;
    writeln!(props, "server-port={}", config.server_port)?;
    writeln!(props, "online-mode={}", config.online_mode)?;
    writeln!(props, "white-list={}", config.white_list)?;
    writeln!(props, "difficulty={}", config.difficulty)?;
    writeln!(props, "pvp={}", config.pvp)?;
    writeln!(props, "spawn-monsters={}", config.spawn_monsters)?;
    writeln!(props, "max-players={}", config.max_players)?;
    writeln!(props, "motd=A Minecraft Server Powered by TitanHost")?;
    writeln!(props, "view-distance={}", config.view_distance)?;
    writeln!(props, "allow-flight={}", config.allow_flight)?;
    writeln!(props, "level-seed={}", config.level_seed)?;
    writeln!(props, "spawn-protection={}", config.spawn_protection)?;
    writeln!(props, "hardcore={}", config.hardcore)?;
    writeln!(props, "generate-structures={}", config.generate_structures)?;
    writeln!(
        props,
        "enable-command-block={}",
        config.enable_command_block
    )?;
    writeln!(props, "gamemode={}", config.gamemode)?;

    // 3. Generate start scripts
    #[cfg(target_os = "windows")]
    {
        let mut bat = std::fs::File::create(format!("{}/run.bat", SERVER_DIR))?;
        writeln!(bat, "@echo off")?;
        writeln!(bat, "java -Xms2G -Xmx4G -jar server.jar nogui")?;
        writeln!(bat, "pause")?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut sh = std::fs::File::create(format!("{}/run.sh", SERVER_DIR))?;
        writeln!(sh, "#!/bin/bash")?;
        writeln!(sh, "java -Xms2G -Xmx4G -jar server.jar nogui")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = sh.metadata()?.permissions();
            perms.set_mode(0o755);
            sh.set_permissions(perms)?;
        }
    }

    Ok(())
}

/// Assemble full Java command and execute process asynchronously
pub fn launch_game_process(
    config: &LauncherConfig,
    tx: std::sync::mpsc::Sender<String>,
) -> Result<
    (
        tokio::process::Child,
        tokio::sync::mpsc::UnboundedSender<String>,
    ),
    AppError,
> {
    let jar_name = format!("{}/server.jar", SERVER_DIR);

    if !Path::new(&jar_name).exists() {
        return Err(AppError::Process(format!(
            "JAR-файл сервера не найден: '{}'",
            jar_name
        )));
    }

    let java_ver = get_java_version_for_minecraft(&config.minecraft_version);
    let java_exec = get_java_executable(java_ver);

    let ram_mx = format!("-Xmx{}G", config.max_ram_gb);
    let ram_ms = format!("-Xms{}G", config.max_ram_gb);

    let mut args = vec![ram_ms, ram_mx];

    if config.aikars_flags {
        for flag in AIKARS_FLAGS {
            args.push(flag.to_string());
        }
    }

    args.push("-jar".to_string());
    args.push("server.jar".to_string());
    args.push("nogui".to_string());

    let full_command = format!("{} {}", java_exec, args.join(" "));
    let _ = tx.send(format!("LOG: [ЗАПУСК] Запуск команды: {}", full_command));

    let log_path = format!("{}/game_output.log", SERVER_DIR);
    let _ = std::fs::remove_file(&log_path);

    let mut child = tokio::process::Command::new(&java_exec)
        .current_dir(SERVER_DIR)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        tokio::spawn(async move {
            while let Some(cmd) = stdin_rx.recv().await {
                let mut cmd_bytes = cmd.into_bytes();
                cmd_bytes.push(b'\n');
                if stdin.write_all(&cmd_bytes).await.is_err() {
                    break;
                }
                if stdin.flush().await.is_err() {
                    break;
                }
            }
        });
    }

    if let Some(stdout) = child.stdout.take() {
        pipe_stream(stdout, tx.clone(), "GAME-OUT", log_path.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        pipe_stream(stderr, tx, "GAME-ERR", log_path);
    }

    Ok((child, stdin_tx))
}
