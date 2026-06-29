use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::FileOptions;

#[derive(Clone, Debug)]
pub struct BackupInfo {
    pub filename: String,
    pub date_str: String,
    pub size_mb: f64,
}

fn add_dir_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    src_dir: &Path,
    base_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stack = vec![src_dir.to_path_buf()];

    while let Some(current_path) = stack.pop() {
        for entry in std::fs::read_dir(&current_path)? {
            let entry = entry?;
            let path = entry.path();

            // Relative path in the zip archive
            let name = path.strip_prefix(base_dir)?;
            let name_str = name.to_string_lossy().to_string();

            if path.is_dir() {
                // Add directory entry
                zip.add_directory(&name_str, FileOptions::default())?;
                stack.push(path);
            } else if path.is_file() {
                // Add file entry
                zip.start_file(&name_str, FileOptions::default())?;
                let mut f = File::open(&path)?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer)?;
                zip.write_all(&buffer)?;
            }
        }
    }
    Ok(())
}

fn add_path_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    path: &Path,
    base_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_dir() {
        // First add the folder itself
        let name = path.strip_prefix(base_dir)?;
        let mut name_str = name.to_string_lossy().to_string();
        if !name_str.ends_with('/') {
            name_str.push('/');
        }
        zip.add_directory(&name_str, FileOptions::default())?;

        // Then recursively add all contents
        add_dir_to_zip(zip, path, base_dir)?;
    } else if path.is_file() {
        let name = path.strip_prefix(base_dir)?;
        let name_str = name.to_string_lossy().to_string();
        zip.start_file(&name_str, FileOptions::default())?;
        let mut f = File::open(path)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        zip.write_all(&buffer)?;
    }
    Ok(())
}

/// Create a ZIP backup of world, mods, plugins, and server.properties
pub fn create_backup_archive(
    server_dir: &str,
    output_zip: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure backups directory exists
    let backups_dir = Path::new(server_dir).join("backups");
    std::fs::create_dir_all(&backups_dir)?;

    let zip_file = File::create(output_zip)?;
    let mut zip = zip::ZipWriter::new(zip_file);

    // We want to backup: world, world_nether, world_the_end, mods, plugins, server.properties
    let targets = vec![
        "world",
        "world_nether",
        "world_the_end",
        "mods",
        "plugins",
        "server.properties",
    ];
    let mut targets_exist = false;
    let base_path = Path::new(server_dir);

    for target in targets {
        let target_path = base_path.join(target);
        if target_path.exists() {
            add_path_to_zip(&mut zip, &target_path, base_path)?;
            targets_exist = true;
        }
    }

    if !targets_exist {
        return Err("No targets (world/mods/plugins/server.properties) found to backup".into());
    }

    zip.finish()?;
    Ok(())
}

/// Restore a ZIP backup by wiping current directories and extracting archive
pub fn restore_backup_archive(
    server_dir: &str,
    zip_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Remove current world, mods, plugins, server.properties
    let targets = vec![
        "world",
        "world_nether",
        "world_the_end",
        "mods",
        "plugins",
        "server.properties",
    ];
    for target in targets {
        let path = Path::new(server_dir).join(target);
        if path.exists() {
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }
        }
    }

    // 2. Extract ZIP
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => Path::new(server_dir).join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        // Get and Set permissions if on unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
            }
        }
    }

    Ok(())
}

/// Scan `/backups/` directory for ZIP backup packages
pub fn list_backups(server_dir: &str) -> Vec<BackupInfo> {
    let backups_dir = Path::new(server_dir).join("backups");
    if !backups_dir.exists() {
        return Vec::new();
    }
    let mut list = Vec::new();
    if let Ok(entries) = std::fs::read_dir(backups_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "zip").unwrap_or(false) {
                let filename = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let size_mb = path
                    .metadata()
                    .map(|m| m.len() as f64 / 1024.0 / 1024.0)
                    .unwrap_or(0.0);

                let date_str = if filename.starts_with("backup_") && filename.len() >= 22 {
                    let parts: Vec<&str> = filename.split('_').collect();
                    if parts.len() >= 3 {
                        let date_part = parts[1];
                        let time_part = parts[2].trim_end_matches(".zip");
                        if date_part.len() == 8 && time_part.len() == 6 {
                            format!(
                                "{}-{}-{} {}:{}:{}",
                                &date_part[0..4],
                                &date_part[4..6],
                                &date_part[6..8],
                                &time_part[0..2],
                                &time_part[2..4],
                                &time_part[4..6]
                            )
                        } else {
                            "Unknown date".to_string()
                        }
                    } else {
                        "Unknown date".to_string()
                    }
                } else {
                    "Unknown date".to_string()
                };

                list.push(BackupInfo {
                    filename,
                    date_str,
                    size_mb,
                });
            }
        }
    }
    list.sort_by(|a, b| b.filename.cmp(&a.filename)); // newest first
    list
}
