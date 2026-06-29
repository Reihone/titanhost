use crate::core::error::AppError;
use crate::core::process::{get_java_executable, get_java_version_for_minecraft};
use crate::core::SERVER_DIR;
use crate::models::{
    FabricInstallerEntry, FabricLoaderEntry, FabricProfileResponse, ModConfig, ModLoader,
    PaperVersionResponse, ServerCoreType, VersionManifest, VersionPackage,
};
use eframe::egui;
use futures_util::StreamExt;
use std::path::Path;
use tokio::io::AsyncWriteExt;

/// Get URL and file name for Forge loader depending on the Minecraft version
pub fn get_forge_info(version: &str) -> (String, String) {
    match version {
        "1.7.10" => (
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.7.10-10.13.4.1614-1.7.10/forge-1.7.10-10.13.4.1614-1.7.10-universal.jar".to_string(),
            "forge-1.7.10.jar".to_string()
        ),
        "1.16.5" => (
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.16.5-36.2.34/forge-1.16.5-36.2.34-universal.jar".to_string(),
            "forge-1.16.5.jar".to_string()
        ),
        "1.20.1" => (
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-universal.jar".to_string(),
            "forge-1.20.1.jar".to_string()
        ),
        _ => (
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.12.2-14.23.5.2860/forge-1.12.2-14.23.5.2860-universal.jar".to_string(),
            "forge-1.12.2.jar".to_string()
        )
    }
}

/// Get URL and file name for Fabric loader depending on the Minecraft version
pub fn get_fabric_info(version: &str) -> (String, String) {
    match version {
        "1.20.1" => (
            "https://meta.fabricmc.net/v2/versions/loader/1.20.1/0.14.22/0.11.2/server/jar"
                .to_string(),
            "fabric-loader-1.20.1.jar".to_string(),
        ),
        "1.16.5" => (
            "https://meta.fabricmc.net/v2/versions/loader/1.16.5/0.14.22/0.11.2/server/jar"
                .to_string(),
            "fabric-loader-1.16.5.jar".to_string(),
        ),
        _ => (
            format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}/0.14.22/0.11.2/server/jar",
                version
            ),
            format!("fabric-loader-{}.jar", version),
        ),
    }
}

/// Helper function to get fallback paper build if API query fails
pub fn get_paper_fallback_build(mc_version: &str) -> String {
    match mc_version {
        "1.12.2" => "1618".to_string(),
        "1.16.5" => "794".to_string(),
        "1.20.1" => "196".to_string(),
        "1.20.4" => "496".to_string(),
        "1.19.2" => "307".to_string(),
        "1.18.2" => "388".to_string(),
        _ => "196".to_string(),
    }
}

/// Extract file name from mod/plugin download URL
pub fn get_filename_from_url(url: &str) -> String {
    if let Some(pos) = url.rfind('/') {
        let filename = &url[pos + 1..];
        let clean_filename = if let Some(query_pos) = filename.find('?') {
            &filename[..query_pos]
        } else {
            filename
        };
        if !clean_filename.is_empty() && clean_filename.ends_with(".jar") {
            return clean_filename.to_string();
        }
    }
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let finalize_res = hasher.finalize();
    let mut hex_part = String::new();
    for byte in &finalize_res[..4] {
        hex_part.push_str(&format!("{:02x}", byte));
    }
    format!("mod_{}.jar", hex_part)
}

/// Calculate SHA-256 hash of a file asynchronously
pub async fn calculate_sha256_async(path: &Path) -> Result<String, std::io::Error> {
    use sha2::{Digest, Sha256};
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Calculate SHA-256 hash of a file synchronously (for blocking UI contexts)
pub fn calculate_sha256_sync(path: &Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Retrieve the status of a local mod/plugin file (Installed, Missing, Changed)
pub fn get_file_status(
    folder: &str,
    hash_cache: &mut std::collections::HashMap<String, (u64, String)>,
    filename: &str,
    expected_hash: &Option<String>,
    enabled: bool,
) -> (String, egui::Color32) {
    if !enabled {
        return ("Выключен".to_string(), egui::Color32::GRAY);
    }
    let path = Path::new(SERVER_DIR).join(folder).join(filename);
    if !path.exists() {
        return (
            "Отсутствует (Будет скачан)".to_string(),
            egui::Color32::from_rgb(230, 100, 100),
        );
    }

    if let Ok(metadata) = std::fs::metadata(&path) {
        let current_size = metadata.len();

        if let Some(&(cached_size, ref cached_hash)) = hash_cache.get(filename) {
            if cached_size == current_size {
                let hash_ref = cached_hash;
                if let Some(ref exp) = expected_hash {
                    if hash_ref == exp {
                        return (
                            "Готов (OK)".to_string(),
                            egui::Color32::from_rgb(100, 230, 100),
                        );
                    } else {
                        return (
                            "Изменен (Будет обновлен)".to_string(),
                            egui::Color32::from_rgb(230, 180, 50),
                        );
                    }
                } else {
                    return (
                        format!("Установлен (хэш: {})", &hash_ref[..8.min(hash_ref.len())]),
                        egui::Color32::from_rgb(100, 200, 250),
                    );
                }
            }
        }

        match calculate_sha256_sync(&path) {
            Ok(hash) => {
                hash_cache.insert(filename.to_string(), (current_size, hash.clone()));
                if let Some(ref exp) = expected_hash {
                    if &hash == exp {
                        return (
                            "Готов (OK)".to_string(),
                            egui::Color32::from_rgb(100, 230, 100),
                        );
                    } else {
                        return (
                            "Изменен (Будет обновлен)".to_string(),
                            egui::Color32::from_rgb(230, 180, 50),
                        );
                    }
                } else {
                    return (
                        format!("Установлен (хэш: {})", &hash[..8.min(hash.len())]),
                        egui::Color32::from_rgb(100, 200, 250),
                    );
                }
            }
            Err(_) => {
                return (
                    "Ошибка чтения файла".to_string(),
                    egui::Color32::from_rgb(230, 100, 100),
                );
            }
        }
    }

    ("Неизвестно".to_string(), egui::Color32::GRAY)
}

/// Download helper utilizing async chunked buffer streaming with UI progress updates
pub async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest_path: &Path,
    label: &str,
    tx: std::sync::mpsc::Sender<String>,
    ctx: egui::Context,
) -> Result<(), AppError> {
    let resp = client.get(url).send().await?;

    if !resp.status().is_success() {
        return Err(AppError::Download(format!(
            "Сервер вернул код {}",
            resp.status()
        )));
    }

    let total_size = resp.content_length();
    let mut file = tokio::fs::File::create(dest_path).await?;

    let mut stream = resp.bytes_stream();
    let mut downloaded = 0u64;
    let mut last_reported_percent = 0;
    let mut last_report_time = std::time::Instant::now();

    let _ = tx.send(format!("LOG: [СЕТЬ] Начало загрузки {}...", label));
    if let Some(total) = total_size {
        let _ = tx.send(format!(
            "LOG: [СЕТЬ] Ожидаемый объем: {:.2} МБ",
            total as f64 / 1024.0 / 1024.0
        ));
    }
    ctx.request_repaint();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if let Some(total) = total_size {
            let percent = (downloaded as f64 / total as f64 * 100.0) as u32;
            let now = std::time::Instant::now();
            if percent >= last_reported_percent + 5
                || now.duration_since(last_report_time).as_secs() >= 1
            {
                let _ = tx.send(format!(
                    "LOG: [СЕТЬ] Загрузка {}: {}% ({:.2} / {:.2} МБ)",
                    label,
                    percent,
                    downloaded as f64 / 1024.0 / 1024.0,
                    total as f64 / 1024.0 / 1024.0
                ));
                ctx.request_repaint();
                last_reported_percent = percent;
                last_report_time = now;
            }
        } else {
            let now = std::time::Instant::now();
            if now.duration_since(last_report_time).as_secs() >= 2 {
                let _ = tx.send(format!(
                    "LOG: [СЕТЬ] Загрузка {}: {:.2} МБ скачано",
                    label,
                    downloaded as f64 / 1024.0 / 1024.0
                ));
                ctx.request_repaint();
                last_report_time = now;
            }
        }
    }

    file.flush().await?;

    let _ = tx.send(format!(
        "LOG: [СЕТЬ] Загрузка {} завершена! ({:.2} МБ)",
        label,
        downloaded as f64 / 1024.0 / 1024.0
    ));
    ctx.request_repaint();
    Ok(())
}

/// Download Java runtime asynchronously and unpack it safely using system Command options
pub fn download_jre(ctx: egui::Context, tx: std::sync::mpsc::Sender<String>, java_ver: u32) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось создать HTTP-клиент для Java: {}",
                    e
                ));
                let _ = tx.send("FINISH_JAVA_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let _ = tx.send(format!("STATUS: Загрузка Java JRE {}...", java_ver));
        ctx.request_repaint();

        let jre_url = format!(
            "https://api.adoptium.net/v3/binary/latest/{}/ga/linux/x64/jre/hotspot/normal/eclipse",
            java_ver
        );
        let tar_path = format!("{}/jre{}_temp.tar.gz", SERVER_DIR, java_ver);
        let target_dir = format!("{}/jre{}", SERVER_DIR, java_ver);
        let extract_dir = format!("{}/jre{}_extract", SERVER_DIR, java_ver);

        if let Err(e) = download_file(
            &client,
            &jre_url,
            Path::new(&tar_path),
            &format!("Java JRE {}", java_ver),
            tx.clone(),
            ctx.clone(),
        )
        .await
        {
            let _ = tx.send(format!(
                "LOG: [ОШИБКА] Загрузка Java JRE {} завершилась с ошибкой: {}",
                java_ver, e
            ));
            let _ = tx.send("FINISH_JAVA_ERROR".to_string());
            ctx.request_repaint();
            return;
        }

        let _ = tx.send(format!(
            "LOG: [СИСТЕМА] Распаковка архива Java JRE {}...",
            java_ver
        ));
        ctx.request_repaint();

        let _ = tokio::fs::create_dir_all(&extract_dir).await;

        match tokio::process::Command::new("tar")
            .arg("-xzf")
            .arg(&tar_path)
            .arg("-C")
            .arg(&extract_dir)
            .output()
            .await
        {
            Ok(out) => {
                if !out.status.success() {
                    let err_str = String::from_utf8_lossy(&out.stderr);
                    let _ = tx.send(format!("LOG: [ОШИБКА] Ошибка распаковки JRE: {}", err_str));
                    let _ = tx.send("FINISH_JAVA_ERROR".to_string());
                    ctx.request_repaint();
                    return;
                }
            }
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Запуск tar для распаковки JRE не удался: {}",
                    e
                ));
                let _ = tx.send("FINISH_JAVA_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        }

        let _ = tokio::fs::remove_dir_all(&target_dir).await;
        let mut renamed = false;

        if let Ok(mut entries) = tokio::fs::read_dir(&extract_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(ft) = entry.file_type().await {
                    if ft.is_dir() {
                        let path = entry.path();
                        if tokio::fs::rename(&path, &target_dir).await.is_ok() {
                            renamed = true;
                            break;
                        }
                    }
                }
            }
        }

        let _ = tokio::fs::remove_dir_all(&extract_dir).await;
        let _ = tokio::fs::remove_file(&tar_path).await;

        if renamed {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let java_exec_path = format!("{}/bin/java", target_dir);
                if let Ok(metadata) = std::fs::metadata(&java_exec_path) {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&java_exec_path, perms);
                }
            }

            let _ = tx.send(format!(
                "LOG: [УСПЕХ] Встроенная Java {} JRE успешно установлена!",
                java_ver
            ));
            let _ = tx.send("FINISH_JAVA_SUCCESS".to_string());
        } else {
            let _ = tx.send(format!(
                "LOG: [ОШИБКА] Не удалось настроить рабочую папку {}.",
                target_dir
            ));
            let _ = tx.send("FINISH_JAVA_ERROR".to_string());
        }
        ctx.request_repaint();
    });
}

/// Download libraries dependencies for client and loaders asynchronously
pub fn download_client_libs(
    ctx: egui::Context,
    tx: std::sync::mpsc::Sender<String>,
    mc_version: String,
    mod_loader: ModLoader,
) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось создать HTTP-клиент для библиотек: {}",
                    e
                ));
                let _ = tx.send("FINISH_LIBS_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let _ = tx.send("STATUS: Загрузка манифеста версий Mojang...".to_string());
        ctx.request_repaint();

        let manifest_url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
        let response = match client.get(manifest_url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось скачать манифест версий: {}",
                    e
                ));
                let _ = tx.send("FINISH_LIBS_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let manifest: VersionManifest = match response.json().await {
            Ok(m) => m,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось распарсить манифест версий: {}",
                    e
                ));
                let _ = tx.send("FINISH_LIBS_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let version_entry = match manifest.versions.iter().find(|v| v.id == mc_version) {
            Some(entry) => entry,
            None => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Версия {} не найдена в манифесте",
                    mc_version
                ));
                let _ = tx.send("FINISH_LIBS_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let _ = tx.send(format!(
            "STATUS: Загрузка манифеста библиотек версии {}...",
            mc_version
        ));
        ctx.request_repaint();

        let pkg_response = match client.get(&version_entry.url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось скачать манифест для версии {}: {}",
                    mc_version, e
                ));
                let _ = tx.send("FINISH_LIBS_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let package: VersionPackage = match pkg_response.json().await {
            Ok(p) => p,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось распарсить манифест для версии {}: {}",
                    mc_version, e
                ));
                let _ = tx.send("FINISH_LIBS_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let libraries = package.libraries;
        let total_libs = libraries.len();
        let _ = tx.send(format!(
            "LOG: [СИСТЕМА] Найдено {} библиотек в манифесте",
            total_libs
        ));
        ctx.request_repaint();

        let mut success_count = 0;
        let mut error_count = 0;

        for (idx, lib) in libraries.iter().enumerate() {
            let lib_num = idx + 1;

            if let Some(ref artifact) = lib.downloads.artifact {
                let dest_path = format!("{}/libraries/{}", SERVER_DIR, artifact.path);
                let path_obj = Path::new(&dest_path);

                if !path_obj.exists() {
                    let _ = tx.send(format!(
                        "STATUS: [{}/{}] Скачивание {}",
                        lib_num, total_libs, lib.name
                    ));
                    ctx.request_repaint();

                    if let Some(parent) = path_obj.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }

                    match client.get(&artifact.url).send().await {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                match tokio::fs::File::create(&dest_path).await {
                                    Ok(mut f) => {
                                        let mut stream = resp.bytes_stream();
                                        let mut write_err = false;
                                        while let Some(chunk_res) = stream.next().await {
                                            match chunk_res {
                                                Ok(chunk) => {
                                                    if let Err(e) = f.write_all(&chunk).await {
                                                        let _ = tx.send(format!(
                                                            "LOG: [ОШИБКА] Ошибка записи {}: {}",
                                                            dest_path, e
                                                        ));
                                                        write_err = true;
                                                        break;
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = tx.send(format!(
                                                        "LOG: [ОШИБКА] Ошибка чтения {}: {}",
                                                        dest_path, e
                                                    ));
                                                    write_err = true;
                                                    break;
                                                }
                                            }
                                        }
                                        let _ = f.flush().await;
                                        if !write_err {
                                            success_count += 1;
                                        } else {
                                            error_count += 1;
                                            let _ = tokio::fs::remove_file(&dest_path).await;
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(format!(
                                            "LOG: [ОШИБКА] Ошибка создания {}: {}",
                                            dest_path, e
                                        ));
                                        error_count += 1;
                                    }
                                }
                            } else {
                                let _ = tx.send(format!(
                                    "LOG: [ОШИБКА] Код {} при скачивании {}",
                                    resp.status(),
                                    artifact.url
                                ));
                                error_count += 1;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(format!(
                                "LOG: [ОШИБКА] Сетевая ошибка при скачивании {}: {}",
                                lib.name, e
                            ));
                            error_count += 1;
                        }
                    }
                } else {
                    success_count += 1;
                }
            }

            let os_key = if cfg!(target_os = "windows") {
                "windows"
            } else if cfg!(target_os = "macos") {
                "osx"
            } else {
                "linux"
            };

            if let Some(ref natives_map) = lib.natives {
                if let Some(classifier_key) = natives_map.get(os_key) {
                    if let Some(ref classifiers) = lib.downloads.classifiers {
                        if let Some(native_artifact) = classifiers.get(classifier_key) {
                            let dest_path =
                                format!("{}/libraries/{}", SERVER_DIR, native_artifact.path);
                            let path_obj = Path::new(&dest_path);

                            let _ = tx.send(format!(
                                "STATUS: [{}/{}] Скачивание нативных библиотек ({})",
                                lib_num, total_libs, classifier_key
                            ));
                            ctx.request_repaint();

                            let mut download_ok = false;
                            if !path_obj.exists() {
                                if let Some(parent) = path_obj.parent() {
                                    let _ = tokio::fs::create_dir_all(parent).await;
                                }

                                match client.get(&native_artifact.url).send().await {
                                    Ok(resp) => {
                                        if resp.status().is_success() {
                                            match tokio::fs::File::create(&dest_path).await {
                                                Ok(mut f) => {
                                                    let mut stream = resp.bytes_stream();
                                                    let mut write_err = false;
                                                    while let Some(chunk_res) = stream.next().await
                                                    {
                                                        match chunk_res {
                                                            Ok(chunk) => {
                                                                if let Err(e) =
                                                                    f.write_all(&chunk).await
                                                                {
                                                                    let _ = tx.send(format!("LOG: [ОШИБКА] Ошибка записи natives {}: {}", dest_path, e));
                                                                    write_err = true;
                                                                    break;
                                                                }
                                                            }
                                                            Err(e) => {
                                                                let _ = tx.send(format!("LOG: [ОШИБКА] Ошибка чтения natives {}: {}", dest_path, e));
                                                                write_err = true;
                                                                break;
                                                            }
                                                        }
                                                    }
                                                    let _ = f.flush().await;
                                                    if !write_err {
                                                        download_ok = true;
                                                    } else {
                                                        let _ = tokio::fs::remove_file(&dest_path)
                                                            .await;
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = tx.send(format!("LOG: [ОШИБКА] Не удалось создать natives {}: {}", dest_path, e));
                                                }
                                            }
                                        } else {
                                            let _ = tx.send(format!(
                                                "LOG: [ОШИБКА] Код {} при скачивании natives {}",
                                                resp.status(),
                                                native_artifact.url
                                            ));
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(format!(
                                            "LOG: [ОШИБКА] Сетевая ошибка natives {}: {}",
                                            lib.name, e
                                        ));
                                    }
                                }
                            } else {
                                download_ok = true;
                            }

                            if download_ok && path_obj.exists() {
                                let _ = tx.send(format!(
                                    "LOG: [СИСТЕМА] Распаковка natives: {}",
                                    classifier_key
                                ));
                                ctx.request_repaint();

                                match tokio::process::Command::new("python3")
                                    .current_dir(SERVER_DIR)
                                    .arg("-c")
                                    .arg(format!(
                                        "import zipfile, os; \
                                         z = zipfile.ZipFile('{}'); \
                                         os.makedirs('natives', exist_ok=True); \
                                         for f in z.infolist(): \
                                             if f.filename.endswith(('.so', '.dll', '.dylib')): \
                                                 f.filename = os.path.basename(f.filename); \
                                                 if f.filename: z.extract(f, 'natives')",
                                        dest_path
                                    ))
                                    .output()
                                    .await
                                {
                                    Ok(out) => {
                                        if !out.status.success() {
                                            let err_str = String::from_utf8_lossy(&out.stderr);
                                            let _ = tx.send(format!("LOG: [ОШИБКА] Ошибка распаковки natives (classifier {}): {}", classifier_key, err_str));
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(format!("LOG: [ОШИБКА] Запуск python3 для распаковки natives не удался: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Additional libraries for Forge 1.12.2 if selected
        if mc_version == "1.12.2" && mod_loader == ModLoader::Forge {
            let _ = tx.send(
                "LOG: [СИСТЕМА] Загрузка дополнительных библиотек Forge 1.12.2...".to_string(),
            );
            ctx.request_repaint();

            let forge_libs = vec![
                ("net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar", "https://libraries.minecraft.net/net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar"),
                ("org/ow2/asm/asm-debug-all/5.2/asm-debug-all-5.2.jar", "https://libraries.minecraft.net/org/ow2/asm/asm-debug-all/5.2/asm-debug-all-5.2.jar"),
                ("com/typesafe/akka/akka-actor_2.11/2.3.3/akka-actor_2.11-2.3.3.jar", "https://maven.minecraftforge.net/com/typesafe/akka/akka-actor_2.11/2.3.3/akka-actor_2.11-2.3.3.jar"),
                ("com/typesafe/config/1.2.1/config-1.2.1.jar", "https://maven.minecraftforge.net/com/typesafe/config/1.2.1/config-1.2.1.jar"),
                ("org/scala-lang/scala-actors-migration_2.11/1.1.0/scala-actors-migration_2.11-1.1.0.jar", "https://maven.minecraftforge.net/org/scala-lang/scala-actors-migration_2.11/1.1.0/scala-actors-migration_2.11-1.1.0.jar"),
                ("org/scala-lang/scala-compiler/2.11.1/scala-compiler-2.11.1.jar", "https://maven.minecraftforge.net/org/scala-lang/scala-compiler/2.11.1/scala-compiler-2.11.1.jar"),
                ("org/scala-lang/scala-library/2.11.1/scala-library-2.11.1.jar", "https://maven.minecraftforge.net/org/scala-lang/scala-library/2.11.1/scala-library-2.11.1.jar"),
                ("org/scala-lang/scala-parser-combinators_2.11/1.0.1/scala-parser-combinators_2.11-1.0.1.jar", "https://maven.minecraftforge.net/org/scala-lang/scala-parser-combinators_2.11/1.0.1/scala-parser-combinators_2.11-1.0.1.jar"),
                ("org/scala-lang/scala-reflect/2.11.1/scala-reflect-2.11.1.jar", "https://maven.minecraftforge.net/org/scala-lang/scala-reflect/2.11.1/scala-reflect-2.11.1.jar"),
                ("org/scala-lang/scala-swing_2.11/1.0.1/scala-swing_2.11-1.0.1.jar", "https://maven.minecraftforge.net/org/scala-lang/scala-swing_2.11/1.0.1/scala-swing_2.11-1.0.1.jar"),
                ("org/scala-lang/scala-xml_2.11/1.0.2/scala-xml_2.11-1.0.2.jar", "https://maven.minecraftforge.net/org/scala-lang/scala-xml_2.11/1.0.2/scala-xml_2.11-1.0.2.jar"),
                ("lzma/lzma/0.0.1/lzma-0.0.1.jar", "https://maven.minecraftforge.net/lzma/lzma/0.0.1/lzma-0.0.1.jar"),
                ("java3d/vecmath/1.5.2/vecmath-1.5.2.jar", "https://libraries.minecraft.net/java3d/vecmath/1.5.2/vecmath-1.5.2.jar"),
                ("net/sf/trove4j/trove4j/3.0.3/trove4j-3.0.3.jar", "https://maven.minecraftforge.net/net/sf/trove4j/trove4j/3.0.3/trove4j-3.0.3.jar"),
                ("org/apache/maven/maven-artifact/3.5.3/maven-artifact-3.5.3.jar", "https://maven.minecraftforge.net/org/apache/maven/maven-artifact/3.5.3/maven-artifact-3.5.3.jar"),
            ];

            let total_forge_libs = forge_libs.len();
            for (idx, (path, url)) in forge_libs.iter().enumerate() {
                let dest_path = format!("{}/libraries/{}", SERVER_DIR, path);
                let path_obj = Path::new(&dest_path);

                if !path_obj.exists() {
                    let _ = tx.send(format!(
                        "STATUS: [{}/{}] Скачивание Forge библиотеки: {}",
                        idx + 1,
                        total_forge_libs,
                        path.split('/').next_back().unwrap_or("")
                    ));
                    ctx.request_repaint();

                    if let Some(parent) = path_obj.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }

                    match client.get(*url).send().await {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                match tokio::fs::File::create(&dest_path).await {
                                    Ok(mut f) => {
                                        let mut stream = resp.bytes_stream();
                                        let mut write_err = false;
                                        while let Some(chunk_res) = stream.next().await {
                                            match chunk_res {
                                                Ok(chunk) => {
                                                    if let Err(e) = f.write_all(&chunk).await {
                                                        let _ = tx.send(format!(
                                                            "LOG: [ОШИБКА] Ошибка записи {}: {}",
                                                            dest_path, e
                                                        ));
                                                        write_err = true;
                                                        break;
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = tx.send(format!(
                                                        "LOG: [ОШИБКА] Ошибка чтения {}: {}",
                                                        dest_path, e
                                                    ));
                                                    write_err = true;
                                                    break;
                                                }
                                            }
                                        }
                                        let _ = f.flush().await;
                                        if !write_err {
                                            success_count += 1;
                                        } else {
                                            error_count += 1;
                                            let _ = tokio::fs::remove_file(&dest_path).await;
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(format!(
                                            "LOG: [ОШИБКА] Не удалось создать файл {}: {}",
                                            dest_path, e
                                        ));
                                        error_count += 1;
                                    }
                                }
                            } else {
                                let _ = tx.send(format!(
                                    "LOG: [ОШИБКА] Код {} при скачивании {}",
                                    resp.status(),
                                    url
                                ));
                                error_count += 1;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(format!(
                                "LOG: [ОШИБКА] Сетевая ошибка при скачивании {}: {}",
                                path, e
                            ));
                            error_count += 1;
                        }
                    }
                } else {
                    success_count += 1;
                }
            }
        }

        // Additional Fabric loader files
        if mod_loader == ModLoader::Fabric {
            let _ = tx.send("LOG: [СИСТЕМА] Загрузка метаданных Fabric loader...".to_string());
            ctx.request_repaint();

            let loader_url = "https://meta.fabricmc.net/v2/versions/loader";
            let loader_ver = match client.get(loader_url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<Vec<FabricLoaderEntry>>().await {
                            Ok(loaders) => loaders
                                .first()
                                .map(|l| l.version.clone())
                                .unwrap_or_else(|| "0.15.11".to_string()),
                            Err(_) => "0.15.11".to_string(),
                        }
                    } else {
                        "0.15.11".to_string()
                    }
                }
                Err(_) => "0.15.11".to_string(),
            };

            let _ = tx.send(format!(
                "LOG: [СИСТЕМА] Загрузка профиля библиотек Fabric для версии {} (loader {})...",
                mc_version, loader_ver
            ));
            ctx.request_repaint();

            let profile_url = format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
                mc_version, loader_ver
            );
            match client.get(&profile_url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<FabricProfileResponse>().await {
                            Ok(profile) => {
                                let total_fab_libs = profile.libraries.len();
                                for (idx, lib) in profile.libraries.iter().enumerate() {
                                    let parts: Vec<&str> = lib.name.split(':').collect();
                                    if parts.len() == 3 {
                                        let group = parts[0].replace('.', "/");
                                        let artifact = parts[1];
                                        let version = parts[2];
                                        let maven_path = format!(
                                            "{}/{}/{}/{}-{}.jar",
                                            group, artifact, version, artifact, version
                                        );
                                        let download_url = format!("{}{}", lib.url, maven_path);
                                        let dest_path =
                                            format!("{}/libraries/{}", SERVER_DIR, maven_path);
                                        let path_obj = Path::new(&dest_path);

                                        if !path_obj.exists() {
                                            let _ =
                                                tx.send(format!(
                                                "STATUS: [{}/{}] Скачивание Fabric библиотеки: {}", 
                                                idx + 1, total_fab_libs, artifact
                                            ));
                                            ctx.request_repaint();

                                            if let Some(parent) = path_obj.parent() {
                                                let _ = tokio::fs::create_dir_all(parent).await;
                                            }

                                            match client.get(&download_url).send().await {
                                                Ok(r) => {
                                                    if r.status().is_success() {
                                                        match tokio::fs::File::create(&dest_path)
                                                            .await
                                                        {
                                                            Ok(mut f) => {
                                                                let mut stream = r.bytes_stream();
                                                                let mut write_err = false;
                                                                while let Some(chunk_res) =
                                                                    stream.next().await
                                                                {
                                                                    match chunk_res {
                                                                        Ok(chunk) => {
                                                                            if let Err(e) = f
                                                                                .write_all(&chunk)
                                                                                .await
                                                                            {
                                                                                let _ = tx.send(format!("LOG: [ОШИБКА] Ошибка записи {}: {}", dest_path, e));
                                                                                write_err = true;
                                                                                break;
                                                                            }
                                                                        }
                                                                        Err(e) => {
                                                                            let _ = tx.send(format!("LOG: [ОШИБКА] Ошибка чтения {}: {}", dest_path, e));
                                                                            write_err = true;
                                                                            break;
                                                                        }
                                                                    }
                                                                }
                                                                let _ = f.flush().await;
                                                                if !write_err {
                                                                    success_count += 1;
                                                                } else {
                                                                    error_count += 1;
                                                                    let _ = tokio::fs::remove_file(
                                                                        &dest_path,
                                                                    )
                                                                    .await;
                                                                }
                                                            }
                                                            Err(e) => {
                                                                let _ = tx.send(format!("LOG: [ОШИБКА] Не удалось создать файл {}: {}", dest_path, e));
                                                                error_count += 1;
                                                            }
                                                        }
                                                    } else {
                                                        let _ = tx.send(format!("LOG: [ОШИБКА] Код {} при скачивании Fabric библиотеки {}", r.status(), download_url));
                                                        error_count += 1;
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = tx.send(format!("LOG: [ОШИБКА] Сетевая ошибка Fabric библиотеки {}: {}", artifact, e));
                                                    error_count += 1;
                                                }
                                            }
                                        } else {
                                            success_count += 1;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(format!(
                                    "LOG: [ОШИБКА] Не удалось распарсить профиль Fabric: {}",
                                    e
                                ));
                                error_count += 1;
                            }
                        }
                    } else {
                        let _ = tx.send(format!(
                            "LOG: [ОШИБКА] Не удалось загрузить профиль Fabric: код {}",
                            resp.status()
                        ));
                        error_count += 1;
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!(
                        "LOG: [ОШИБКА] Сетевая ошибка при загрузке профиля Fabric: {}",
                        e
                    ));
                    error_count += 1;
                }
            }
        }

        let _ = tx.send(format!(
            "LOG: [УСПЕХ] Установка библиотек завершена. Успешно: {}, Ошибок: {}",
            success_count, error_count
        ));
        if error_count == 0 {
            let _ = tx.send("FINISH_LIBS_SUCCESS".to_string());
        } else {
            let _ = tx.send("FINISH_LIBS_ERROR".to_string());
        }
        ctx.request_repaint();
    });
}

/// Download Minecraft vanilla version jar
pub fn download_version(ctx: egui::Context, tx: std::sync::mpsc::Sender<String>, version: String) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось создать HTTP-клиент для версии: {}",
                    e
                ));
                let _ = tx.send("FINISH_VERSION_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let _ = tx.send("STATUS: Загрузка манифеста версий...".to_string());
        ctx.request_repaint();

        let manifest_url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
        let response = match client.get(manifest_url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось скачать манифест версий: {}",
                    e
                ));
                let _ = tx.send("FINISH_VERSION_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let manifest: crate::models::mojang::VersionManifest = match response.json().await {
            Ok(m) => m,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось распарсить манифест версий: {}",
                    e
                ));
                let _ = tx.send("FINISH_VERSION_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let version_entry = match manifest.versions.iter().find(|v| v.id == version) {
            Some(entry) => entry,
            None => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Версия {} не найдена в манифесте версий Mojang",
                    version
                ));
                let _ = tx.send("FINISH_VERSION_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let _ = tx.send(format!("STATUS: Загрузка метаданных версии {}...", version));
        ctx.request_repaint();

        let pkg_response = match client.get(&version_entry.url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось скачать метаданные версии {}: {}",
                    version, e
                ));
                let _ = tx.send("FINISH_VERSION_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let package: crate::models::mojang::VersionPackage = match pkg_response.json().await {
            Ok(pkg) => pkg,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось распарсить метаданные версии {}: {}",
                    version, e
                ));
                let _ = tx.send("FINISH_VERSION_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let url = package.downloads.client.url;
        let filename = format!("{}/{}.jar", SERVER_DIR, version);
        let _ = tx.send(format!("STATUS: Скачивание Minecraft {}...", version));
        ctx.request_repaint();

        if let Err(e) = download_file(
            &client,
            &url,
            Path::new(&filename),
            &format!("Minecraft {}", version),
            tx.clone(),
            ctx.clone(),
        )
        .await
        {
            let _ = tx.send(format!("LOG: [ОШИБКА] Загрузка версии прервана: {}", e));
            let _ = tx.send("FINISH_VERSION_ERROR".to_string());
        } else {
            let _ = tx.send(format!(
                "LOG: [УСПЕХ] Версия {} успешно скачана и установлена как '{}'!",
                version, filename
            ));
            let _ = tx.send("FINISH_VERSION_SUCCESS".to_string());
        }
        ctx.request_repaint();
    });
}

/// Download Minecraft server cores dynamically
pub fn download_server_core(
    ctx: egui::Context,
    tx: std::sync::mpsc::Sender<String>,
    core_type: ServerCoreType,
    mc_version: String,
    core_version: String,
    filename: String,
    core_name: String,
) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось создать HTTP-клиент для {}: {}",
                    core_name, e
                ));
                let _ = tx.send("FINISH_SERVER_CORE_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let _ = tx.send(format!("STATUS: Определение ссылки для {}...", core_name));
        ctx.request_repaint();

        let url = match core_type {
            ServerCoreType::Vanilla => {
                let manifest_url =
                    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
                let response = match client.get(manifest_url).send().await {
                    Ok(resp) => resp,
                    Err(e) => {
                        let _ = tx.send(format!(
                            "LOG: [ОШИБКА] Не удалось скачать манифест версий Mojang: {}",
                            e
                        ));
                        let _ = tx.send("FINISH_SERVER_CORE_ERROR".to_string());
                        ctx.request_repaint();
                        return;
                    }
                };
                let manifest: VersionManifest = match response.json().await {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = tx.send(format!(
                            "LOG: [ОШИБКА] Не удалось распарсить манифест версий Mojang: {}",
                            e
                        ));
                        let _ = tx.send("FINISH_SERVER_CORE_ERROR".to_string());
                        ctx.request_repaint();
                        return;
                    }
                };

                let version_entry = match manifest.versions.iter().find(|v| v.id == mc_version) {
                    Some(entry) => entry,
                    None => {
                        let _ = tx.send(format!(
                            "LOG: [ОШИБКА] Версия {} не найдена в манифесте Mojang",
                            mc_version
                        ));
                        let _ = tx.send("FINISH_SERVER_CORE_ERROR".to_string());
                        ctx.request_repaint();
                        return;
                    }
                };

                let pkg_response = match client.get(&version_entry.url).send().await {
                    Ok(resp) => resp,
                    Err(e) => {
                        let _ = tx.send(format!(
                            "LOG: [ОШИБКА] Не удалось скачать метаданные версии {}: {}",
                            mc_version, e
                        ));
                        let _ = tx.send("FINISH_SERVER_CORE_ERROR".to_string());
                        ctx.request_repaint();
                        return;
                    }
                };
                let package: VersionPackage = match pkg_response.json().await {
                    Ok(pkg) => pkg,
                    Err(e) => {
                        let _ = tx.send(format!(
                            "LOG: [ОШИБКА] Не удалось распарсить метаданные версии {}: {}",
                            mc_version, e
                        ));
                        let _ = tx.send("FINISH_SERVER_CORE_ERROR".to_string());
                        ctx.request_repaint();
                        return;
                    }
                };

                match package.downloads.server {
                    Some(server_artifact) => server_artifact.url,
                    None => {
                        let _ = tx.send(format!("LOG: [ОШИБКА] Серверный файл для версии {} отсутствует в манифесте Mojang", mc_version));
                        let _ = tx.send("FINISH_SERVER_CORE_ERROR".to_string());
                        ctx.request_repaint();
                        return;
                    }
                }
            }
            ServerCoreType::Paper => {
                let build = if core_version != "Последний стабильный" && !core_version.is_empty()
                {
                    core_version.clone()
                } else {
                    let api_url = format!(
                        "https://api.papermc.io/v2/projects/paper/versions/{}",
                        mc_version
                    );
                    match client.get(&api_url).send().await {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                match resp.json::<PaperVersionResponse>().await {
                                    Ok(parsed) => {
                                        if let Some(last_build) = parsed.builds.last() {
                                            last_build.to_string()
                                        } else {
                                            let _ = tx.send(format!("LOG: [ПРЕДУПРЕЖДЕНИЕ] Пустой список билдов для Paper {}, используем дефолт", mc_version));
                                            get_paper_fallback_build(&mc_version)
                                        }
                                    }
                                    Err(_) => get_paper_fallback_build(&mc_version),
                                }
                            } else {
                                get_paper_fallback_build(&mc_version)
                            }
                        }
                        Err(_) => get_paper_fallback_build(&mc_version),
                    }
                };

                format!(
                    "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/paper-{}-{}.jar",
                    mc_version, build, mc_version, build
                )
            }
            ServerCoreType::Fabric => {
                let loader_url = "https://meta.fabricmc.net/v2/versions/loader";
                let loader_ver = match client.get(loader_url).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            match resp.json::<Vec<FabricLoaderEntry>>().await {
                                Ok(loaders) => loaders
                                    .first()
                                    .map(|l| l.version.clone())
                                    .unwrap_or_else(|| "0.19.3".to_string()),
                                Err(_) => "0.19.3".to_string(),
                            }
                        } else {
                            "0.19.3".to_string()
                        }
                    }
                    Err(_) => "0.19.3".to_string(),
                };

                let installer_url = "https://meta.fabricmc.net/v2/versions/installer";
                let installer_ver = match client.get(installer_url).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            match resp.json::<Vec<FabricInstallerEntry>>().await {
                                Ok(installers) => installers
                                    .first()
                                    .map(|i| i.version.clone())
                                    .unwrap_or_else(|| "1.1.1".to_string()),
                                Err(_) => "1.1.1".to_string(),
                            }
                        } else {
                            "1.1.1".to_string()
                        }
                    }
                    Err(_) => "1.1.1".to_string(),
                };

                format!(
                    "https://meta.fabricmc.net/v2/versions/loader/{}/{}/{}/server/jar",
                    mc_version, loader_ver, installer_ver
                )
            }
            ServerCoreType::NeoForge => {
                // Return dummy NeoForge URL (handled as NeoForge installer)
                format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{0}/neoforge-{0}-installer.jar", mc_version)
            }
            ServerCoreType::Forge => {
                let forge_ver = if core_version != "Последний стабильный"
                    && !core_version.is_empty()
                {
                    core_version.clone()
                } else {
                    match mc_version.as_str() {
                        "1.12.2" => "14.23.5.2860".to_string(),
                        "1.16.5" => "36.2.34".to_string(),
                        "1.20.1" => "47.2.0".to_string(),
                        "1.7.10" => "10.13.4.1614-1.7.10".to_string(),
                        _ => "14.23.5.2860".to_string(),
                    }
                };
                format!(
                    "https://maven.minecraftforge.net/net/minecraftforge/forge/{0}-{1}/forge-{0}-{1}-installer.jar",
                    mc_version, forge_ver
                )
            }
        };

        let core_filepath = format!("{}/{}", SERVER_DIR, filename);
        let _ = tx.send(format!(
            "STATUS: Установка серверного ядра {}...",
            core_name
        ));
        ctx.request_repaint();

        if let Err(e) = download_file(
            &client,
            &url,
            Path::new(&core_filepath),
            &core_name,
            tx.clone(),
            ctx.clone(),
        )
        .await
        {
            let _ = tx.send(format!(
                "LOG: [ОШИБКА] Загрузка ядра сервера прервана: {}",
                e
            ));
            let _ = tx.send("FINISH_SERVER_CORE_ERROR".to_string());
        } else {
            if core_type == ServerCoreType::Forge {
                let _ = tx.send("LOG: [СИСТЕМА] Запуск установщика Forge сервера...".to_string());
                ctx.request_repaint();

                let java_ver = get_java_version_for_minecraft(&mc_version);
                let java_exec = get_java_executable(java_ver);

                match tokio::process::Command::new(&java_exec)
                    .current_dir(SERVER_DIR)
                    .arg("-jar")
                    .arg(&core_filepath)
                    .arg("--installServer")
                    .output()
                    .await
                {
                    Ok(out) => {
                        if !out.status.success() {
                            let err_str = String::from_utf8_lossy(&out.stderr);
                            let _ = tx.send(format!(
                                "LOG: [ОШИБКА] Ошибка установщика Forge: {}",
                                err_str
                            ));
                            let _ = tx.send("FINISH_SERVER_CORE_ERROR".to_string());
                            ctx.request_repaint();
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(format!(
                            "LOG: [ОШИБКА] Не удалось запустить установщик Forge: {}",
                            e
                        ));
                        let _ = tx.send("FINISH_SERVER_CORE_ERROR".to_string());
                        ctx.request_repaint();
                        return;
                    }
                }

                let mut found_jar = false;
                if let Ok(mut entries) = tokio::fs::read_dir(SERVER_DIR).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.starts_with("forge-")
                                && name.contains(&mc_version)
                                && name.ends_with(".jar")
                                && !name.contains("installer")
                                && !name.starts_with("forge-server-")
                            {
                                let _ = tokio::fs::copy(
                                    entry.path(),
                                    format!("{}/server.jar", SERVER_DIR),
                                )
                                .await;
                                let _ = tokio::fs::copy(entry.path(), &core_filepath).await;
                                found_jar = true;
                                break;
                            }
                        }
                    }
                }

                if !found_jar {
                    let _ = tx.send("LOG: [СИСТЕМА] Не нашли отдельного jar, возможно это новая версия Forge с запускающим скриптом.".to_string());
                }

                let _ = tokio::fs::remove_file(format!("{}.log", core_filepath)).await;
                let _ = tx.send(format!(
                    "LOG: [УСПЕХ] Серверное ядро {} успешно установлено!",
                    core_name
                ));
                let _ = tx.send("FINISH_SERVER_CORE_SUCCESS".to_string());
            } else {
                if let Err(err) =
                    tokio::fs::copy(&core_filepath, format!("{}/server.jar", SERVER_DIR)).await
                {
                    let _ = tx.send(format!(
                        "LOG: [ПРЕДУПРЕЖДЕНИЕ] Не удалось скопировать {} в server.jar: {}",
                        filename, err
                    ));
                } else {
                    let _ =
                        tx.send("LOG: [СИСТЕМА] Создана копия ядра как 'server.jar'".to_string());
                }
                let _ = tx.send(format!(
                    "LOG: [УСПЕХ] Серверное ядро {} успешно установлено!",
                    core_name
                ));
                let _ = tx.send("FINISH_SERVER_CORE_SUCCESS".to_string());
            }
        }
        ctx.request_repaint();
    });
}

/// Download Minecraft loaders (Forge/Fabric/NeoForge installer) asynchronously
pub fn download_loader(
    ctx: egui::Context,
    tx: std::sync::mpsc::Sender<String>,
    url: String,
    filename: String,
    loader_name: String,
) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось создать HTTP-клиент для {}: {}",
                    loader_name, e
                ));
                let _ = tx.send("FINISH_LOADER_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let _ = tx.send(format!("STATUS: Установка {}...", loader_name));
        ctx.request_repaint();

        let loader_filepath = format!("{}/{}", SERVER_DIR, filename);
        if let Err(e) = download_file(
            &client,
            &url,
            Path::new(&loader_filepath),
            &loader_name,
            tx.clone(),
            ctx.clone(),
        )
        .await
        {
            let _ = tx.send(format!(
                "LOG: [ОШИБКА] Загрузка ядра {} прервана: {}",
                loader_name, e
            ));
            let _ = tx.send("FINISH_LOADER_ERROR".to_string());
        } else {
            let _ = tx.send(format!(
                "LOG: [УСПЕХ] {} успешно скачан и установлен как '{}'!",
                loader_name, filename
            ));
            let _ = tx.send("FINISH_LOADER_SUCCESS".to_string());
        }
        ctx.request_repaint();
    });
}

/// Sync mod packages listed in active configuration
pub fn sync_mods(
    ctx: egui::Context,
    tx: std::sync::mpsc::Sender<String>,
    mods: Vec<ModConfig>,
    _launch_after: bool,
) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось создать HTTP-клиент: {}",
                    e
                ));
                let _ = tx.send("FINISH_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let _ = tx.send("STATUS: Синхронизация модов...".to_string());
        let _ = tx.send("LOG: [СЕТЬ] Проверка списка модов...".to_string());
        ctx.request_repaint();

        let mods_dir = format!("{}/mods", SERVER_DIR);
        if let Err(e) = tokio::fs::create_dir_all(&mods_dir).await {
            let _ = tx.send(format!("LOG: [ОШИБКА] Не удалось создать mods/: {}", e));
            let _ = tx.send("FINISH_ERROR".to_string());
            ctx.request_repaint();
            return;
        }

        let mut has_errors = false;
        let mut updated_mods = Vec::new();

        for mut m in mods {
            let path = Path::new(&mods_dir).join(&m.filename);
            let disabled_path = Path::new(&mods_dir).join(format!("{}.disabled", m.filename));

            if !m.enabled {
                if path.exists() {
                    let _ = tokio::fs::rename(&path, &disabled_path).await;
                    let _ = tx.send(format!(
                        "LOG: [СИСТЕМА] Выключен мод: '{}' (переименован в .disabled)",
                        m.filename
                    ));
                }
                m.sha256 = None;
                updated_mods.push(m);
                ctx.request_repaint();
                continue;
            }

            if disabled_path.exists() && !path.exists() {
                let _ = tokio::fs::rename(&disabled_path, &path).await;
                let _ = tx.send(format!(
                    "LOG: [СИСТЕМА] Включен мод: '{}' (переименован обратно в .jar)",
                    m.filename
                ));
            }

            let file_exists = path.exists();
            let mut download_needed = !file_exists;
            let mut current_hash = None;

            if file_exists {
                match calculate_sha256_async(&path).await {
                    Ok(hash) => {
                        current_hash = Some(hash.clone());
                        if !m.allow_update {
                            let _ =
                                tx.send(format!("LOG: [ИНФО] '{}' локально проверен.", m.filename));
                            m.sha256 = Some(hash);
                            updated_mods.push(m);
                            ctx.request_repaint();
                            continue;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(format!(
                            "LOG: [ПРЕДУПРЕЖДЕНИЕ] Ошибка хэширования '{}': {}. Перекачиваем.",
                            m.filename, e
                        ));
                        download_needed = true;
                    }
                }
            }

            if download_needed || m.allow_update {
                let _ = tx.send(format!("LOG: [СЕТЬ] Загрузка '{}'...", m.filename));
                ctx.request_repaint();
                match client.get(&m.url).send().await {
                    Ok(resp) => {
                        if !resp.status().is_success() {
                            let _ = tx.send(format!(
                                "LOG: [ОШИБКА] Сервер вернул {} при скачивании {}",
                                resp.status(),
                                m.filename
                            ));
                            if file_exists {
                                let _ = tx.send(format!(
                                    "LOG: [ПРЕДУПРЕЖДЕНИЕ] Используется старая копия '{}'.",
                                    m.filename
                                ));
                                m.sha256 = current_hash;
                                updated_mods.push(m);
                            } else {
                                has_errors = true;
                            }
                            ctx.request_repaint();
                            continue;
                        }

                        match resp.bytes().await {
                            Ok(bytes) => {
                                use sha2::{Digest, Sha256};
                                let mut hasher = Sha256::new();
                                hasher.update(&bytes);
                                let new_hash = format!("{:x}", hasher.finalize());

                                let is_changed = match current_hash {
                                    Some(ref h) => h != &new_hash,
                                    None => true,
                                };

                                if is_changed {
                                    match tokio::fs::File::create(&path).await {
                                        Ok(mut f) => {
                                            if let Err(e) = f.write_all(&bytes).await {
                                                let _ = tx.send(format!(
                                                    "LOG: [ОШИБКА] Запись {}: {}",
                                                    m.filename, e
                                                ));
                                                has_errors = true;
                                            } else {
                                                let _ = tx.send(format!(
                                                    "LOG: [УСПЕХ] '{}' загружен. Хэш: {}",
                                                    m.filename,
                                                    &new_hash[..8.min(new_hash.len())]
                                                ));
                                                m.sha256 = Some(new_hash);
                                            }
                                        }
                                        Err(e) => {
                                            let _ = tx.send(format!(
                                                "LOG: [ОШИБКА] Создание файла {}: {}",
                                                m.filename, e
                                            ));
                                            has_errors = true;
                                        }
                                    }
                                } else {
                                    let _ = tx.send(format!(
                                        "LOG: [ИНФО] '{}' не изменился (хэш совпадает).",
                                        m.filename
                                    ));
                                    m.sha256 = Some(new_hash);
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(format!(
                                    "LOG: [ОШИБКА] Чтение байт '{}': {}",
                                    m.filename, e
                                ));
                                has_errors = true;
                                if file_exists {
                                    m.sha256 = current_hash;
                                    updated_mods.push(m);
                                }
                                ctx.request_repaint();
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        let _ =
                            tx.send(format!("LOG: [ОШИБКА] Ошибка сети '{}': {}", m.filename, e));
                        if file_exists {
                            let _ = tx.send(format!("LOG: [ПРЕДУПРЕЖДЕНИЕ] Офлайн режим. Сохранена локальная копия '{}'.", m.filename));
                            m.sha256 = current_hash;
                            updated_mods.push(m);
                        } else {
                            has_errors = true;
                        }
                        ctx.request_repaint();
                        continue;
                    }
                }
            }
            updated_mods.push(m);
            ctx.request_repaint();
        }

        if let Ok(json_str) = serde_json::to_string(&updated_mods) {
            let _ = tx.send(format!("UPDATE_MODS:{}", json_str));
        }

        if has_errors {
            let _ = tx.send("FINISH_ERROR".to_string());
        } else {
            let _ = tx.send("FINISH_SUCCESS".to_string());
        }
        ctx.request_repaint();
    });
}

/// Sync plugin files listed in active configuration
pub fn sync_plugins(
    ctx: egui::Context,
    tx: std::sync::mpsc::Sender<String>,
    plugins: Vec<ModConfig>,
) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(format!(
                    "LOG: [ОШИБКА] Не удалось создать HTTP-клиент для плагинов: {}",
                    e
                ));
                let _ = tx.send("FINISH_PLUGINS_ERROR".to_string());
                ctx.request_repaint();
                return;
            }
        };

        let _ = tx.send("STATUS: Синхронизация плагинов...".to_string());
        let _ = tx.send("LOG: [СЕТЬ] Проверка списка плагинов...".to_string());
        ctx.request_repaint();

        let plugins_dir = format!("{}/plugins", SERVER_DIR);
        if let Err(e) = tokio::fs::create_dir_all(&plugins_dir).await {
            let _ = tx.send(format!("LOG: [ОШИБКА] Не удалось создать plugins/: {}", e));
            let _ = tx.send("FINISH_PLUGINS_ERROR".to_string());
            ctx.request_repaint();
            return;
        }

        let mut has_errors = false;
        let mut updated_plugins = Vec::new();

        for mut p in plugins {
            let path = Path::new(&plugins_dir).join(&p.filename);
            let disabled_path = Path::new(&plugins_dir).join(format!("{}.disabled", p.filename));

            if !p.enabled {
                if path.exists() {
                    let _ = tokio::fs::rename(&path, &disabled_path).await;
                    let _ = tx.send(format!(
                        "LOG: [СИСТЕМА] Выключен плагин: '{}' (переименован в .disabled)",
                        p.filename
                    ));
                }
                p.sha256 = None;
                updated_plugins.push(p);
                ctx.request_repaint();
                continue;
            }

            if disabled_path.exists() && !path.exists() {
                let _ = tokio::fs::rename(&disabled_path, &path).await;
                let _ = tx.send(format!(
                    "LOG: [СИСТЕМА] Включен плагин: '{}' (переименован обратно в .jar)",
                    p.filename
                ));
            }

            let file_exists = path.exists();
            let mut download_needed = !file_exists;
            let mut current_hash = None;

            if file_exists {
                match calculate_sha256_async(&path).await {
                    Ok(hash) => {
                        current_hash = Some(hash.clone());
                        if !p.allow_update {
                            let _ = tx.send(format!(
                                "LOG: [ИНФО] Плагин '{}' локально проверен.",
                                p.filename
                            ));
                            p.sha256 = Some(hash);
                            updated_plugins.push(p);
                            ctx.request_repaint();
                            continue;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(format!("LOG: [ПРЕДУПРЕЖДЕНИЕ] Ошибка хэширования плагина '{}': {}. Перекачиваем.", p.filename, e));
                        download_needed = true;
                    }
                }
            }

            if download_needed || p.allow_update {
                let _ = tx.send(format!("LOG: [СЕТЬ] Загрузка плагина '{}'...", p.filename));
                ctx.request_repaint();
                match client.get(&p.url).send().await {
                    Ok(resp) => {
                        if !resp.status().is_success() {
                            let _ = tx.send(format!(
                                "LOG: [ОШИБКА] Сервер вернул {} при скачивании плагина {}",
                                resp.status(),
                                p.filename
                            ));
                            if file_exists {
                                let _ = tx.send(format!(
                                    "LOG: [ПРЕДУПРЕЖДЕНИЕ] Используется старая копия плагина '{}'.",
                                    p.filename
                                ));
                                p.sha256 = current_hash;
                                updated_plugins.push(p);
                            } else {
                                has_errors = true;
                            }
                            ctx.request_repaint();
                            continue;
                        }

                        match resp.bytes().await {
                            Ok(bytes) => {
                                use sha2::{Digest, Sha256};
                                let mut hasher = Sha256::new();
                                hasher.update(&bytes);
                                let new_hash = format!("{:x}", hasher.finalize());

                                let is_changed = match current_hash {
                                    Some(ref h) => h != &new_hash,
                                    None => true,
                                };

                                if is_changed {
                                    match tokio::fs::File::create(&path).await {
                                        Ok(mut f) => {
                                            if let Err(e) = f.write_all(&bytes).await {
                                                let _ = tx.send(format!(
                                                    "LOG: [ОШИБКА] Запись плагина {}: {}",
                                                    p.filename, e
                                                ));
                                                has_errors = true;
                                            } else {
                                                let _ = tx.send(format!(
                                                    "LOG: [УСПЕХ] Плагин '{}' загружен.",
                                                    p.filename
                                                ));
                                                p.sha256 = Some(new_hash);
                                            }
                                        }
                                        Err(e) => {
                                            let _ = tx.send(format!(
                                                "LOG: [ОШИБКА] Создание файла плагина {}: {}",
                                                p.filename, e
                                            ));
                                            has_errors = true;
                                        }
                                    }
                                } else {
                                    let _ = tx.send(format!(
                                        "LOG: [ИНФО] Плагин '{}' не изменился.",
                                        p.filename
                                    ));
                                    p.sha256 = Some(new_hash);
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(format!(
                                    "LOG: [ОШИБКА] Чтение байт плагина '{}': {}",
                                    p.filename, e
                                ));
                                has_errors = true;
                                if file_exists {
                                    p.sha256 = current_hash;
                                    updated_plugins.push(p);
                                }
                                ctx.request_repaint();
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(format!(
                            "LOG: [ОШИБКА] Ошибка сети при загрузке плагина '{}': {}",
                            p.filename, e
                        ));
                        if file_exists {
                            p.sha256 = current_hash;
                            updated_plugins.push(p);
                        } else {
                            has_errors = true;
                        }
                        ctx.request_repaint();
                        continue;
                    }
                }
            }
            updated_plugins.push(p);
            ctx.request_repaint();
        }

        if let Ok(json_str) = serde_json::to_string(&updated_plugins) {
            let _ = tx.send(format!("UPDATE_PLUGINS:{}", json_str));
        }

        if has_errors {
            let _ = tx.send("FINISH_PLUGINS_ERROR".to_string());
        } else {
            let _ = tx.send("FINISH_PLUGINS_SUCCESS".to_string());
        }
        ctx.request_repaint();
    });
}
