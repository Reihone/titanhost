use crate::core::downloader::{
    download_client_libs, download_jre, download_loader, download_server_core, download_version,
    get_fabric_info, get_file_status, get_filename_from_url, get_forge_info, sync_mods,
    sync_plugins,
};
use crate::core::process::get_java_version_for_minecraft;
use crate::core::SERVER_DIR;
use crate::models::{ModConfig, ModLoader, ServerCoreType};
use crate::ui::LauncherApp;
use eframe::egui;
use std::path::Path;

/// Render the Modding sub-panels (Core/Versions download, mod download, plugin download, Java/libs downloads)
pub fn draw_modding_tab(app: &mut LauncherApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.horizontal(|ui| {
        ui.selectable_value(
            &mut app.modding_sub_tab,
            crate::models::ModdingSubTab::CoresAndVersions,
            "⚙️ Ядра & Версии",
        );
        if ui
            .selectable_value(
                &mut app.modding_sub_tab,
                crate::models::ModdingSubTab::Mods,
                "📦 Моды",
            )
            .clicked()
        {
            scan_local_mods(app);
        }
        if ui
            .selectable_value(
                &mut app.modding_sub_tab,
                crate::models::ModdingSubTab::Plugins,
                "🧩 Плагины",
            )
            .clicked()
        {
            scan_local_plugins(app);
        }
        ui.selectable_value(
            &mut app.modding_sub_tab,
            crate::models::ModdingSubTab::Dependencies,
            "📥 Зависимости",
        );
    });
    ui.separator();
    ui.add_space(4.0);

    match app.modding_sub_tab {
        crate::models::ModdingSubTab::CoresAndVersions => {
            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("⚙️ Ядро Minecraft (Клиент)").strong());
                        ui.add_space(4.0);
                        ui.radio_value(&mut app.config.mod_loader, ModLoader::Vanilla, "Vanilla (Чистая)");
                        ui.radio_value(&mut app.config.mod_loader, ModLoader::Forge, "Forge");
                        
                        let mc_version = &app.config.minecraft_version;
                        let has_fabric = mc_version != "1.12.2" && mc_version != "1.7.10";
                        if has_fabric {
                            ui.radio_value(&mut app.config.mod_loader, ModLoader::Fabric, "Fabric");
                        } else if app.config.mod_loader == ModLoader::Fabric {
                            app.config.mod_loader = ModLoader::Forge;
                        }

                        let has_neoforge = mc_version == "1.20.4";
                        if has_neoforge {
                            ui.radio_value(&mut app.config.mod_loader, ModLoader::NeoForge, "NeoForge");
                        } else if app.config.mod_loader == ModLoader::NeoForge {
                            app.config.mod_loader = ModLoader::Forge;
                        }
                    });
                });

                columns[1].vertical(|ui| {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("📥 Скачивание Клиента & Лоадеров").strong());
                        ui.add_space(4.0);

                        let vanilla_jar = format!("{}/{}.jar", SERVER_DIR, app.config.minecraft_version);
                        let version_installed = Path::new(&vanilla_jar).exists();
                        ui.horizontal(|ui| {
                            ui.label(format!("Базовая версия {}:", app.config.minecraft_version));
                            if version_installed {
                                ui.colored_label(egui::Color32::LIGHT_GREEN, "Установлена");
                            } else {
                                if crate::core::process::get_minecraft_version_url(&app.config.minecraft_version).is_some() {
                                    if ui.button("📥 Скачать").clicked()
                                        && !app.is_downloading_version {
                                            app.is_downloading_version = true;
                                            download_version(ctx.clone(), app.tx.clone(), app.config.minecraft_version.clone());
                                        }
                                } else {
                                    ui.colored_label(egui::Color32::LIGHT_RED, "Нет автоссылки");
                                }
                            }
                        });

                        match app.config.mod_loader {
                            ModLoader::Vanilla => {}
                            ModLoader::Forge => {
                                let (url, filename) = get_forge_info(&app.config.minecraft_version);
                                let loader_path = format!("{}/{}", SERVER_DIR, filename);
                                let loader_installed = Path::new(&loader_path).exists();
                                ui.horizontal(|ui| {
                                    ui.label("Установка Forge:");
                                    if loader_installed {
                                        ui.colored_label(egui::Color32::LIGHT_GREEN, "Установлен");
                                    } else {
                                        if ui.button("📥 Скачать Forge").clicked()
                                            && !app.is_downloading_loader {
                                                app.is_downloading_loader = true;
                                                download_loader(ctx.clone(), app.tx.clone(), url, filename, "Forge".to_string());
                                            }
                                    }
                                });
                            }
                            ModLoader::Fabric => {
                                let (url, filename) = get_fabric_info(&app.config.minecraft_version);
                                let loader_path = format!("{}/{}", SERVER_DIR, filename);
                                let loader_installed = Path::new(&loader_path).exists();
                                ui.horizontal(|ui| {
                                    ui.label("Установка Fabric:");
                                    if loader_installed {
                                        ui.colored_label(egui::Color32::LIGHT_GREEN, "Установлен");
                                    } else {
                                        if ui.button("📥 Скачать Fabric").clicked()
                                            && !app.is_downloading_loader {
                                                app.is_downloading_loader = true;
                                                download_loader(ctx.clone(), app.tx.clone(), url, filename, "Fabric".to_string());
                                            }
                                    }
                                });
                            }
                            ModLoader::NeoForge => {
                                let filename = format!("neoforge-{}.jar", app.config.minecraft_version);
                                let loader_path = format!("{}/{}", SERVER_DIR, filename);
                                let loader_installed = Path::new(&loader_path).exists();
                                ui.horizontal(|ui| {
                                    ui.label("Установка NeoForge:");
                                    if loader_installed {
                                        ui.colored_label(egui::Color32::LIGHT_GREEN, "Установлен");
                                    } else {
                                        let url = format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{0}/neoforge-{0}-installer.jar", app.config.minecraft_version);
                                        if ui.button("📥 Скачать NeoForge").clicked()
                                            && !app.is_downloading_loader {
                                                app.is_downloading_loader = true;
                                                download_loader(ctx.clone(), app.tx.clone(), url, filename, "NeoForge".to_string());
                                            }
                                    }
                                });
                            }
                        }
                    });
                });
            });

            ui.separator();
            ui.add_space(4.0);

            ui.group(|ui| {
                ui.label(egui::RichText::new("🖥️ Образ сервера (Ядро сервера)").strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Тип ядра сервера:");
                    ui.radio_value(
                        &mut app.config.server_core_type,
                        ServerCoreType::Vanilla,
                        "Vanilla Server",
                    );
                    ui.radio_value(
                        &mut app.config.server_core_type,
                        ServerCoreType::Paper,
                        "Paper (Плагины)",
                    );
                    ui.radio_value(
                        &mut app.config.server_core_type,
                        ServerCoreType::Forge,
                        "Forge Server",
                    );

                    let mc_version = &app.config.minecraft_version;
                    let has_fabric = mc_version != "1.12.2" && mc_version != "1.7.10";
                    if has_fabric {
                        ui.radio_value(
                            &mut app.config.server_core_type,
                            ServerCoreType::Fabric,
                            "Fabric Server",
                        );
                    } else if app.config.server_core_type == ServerCoreType::Fabric {
                        app.config.server_core_type = ServerCoreType::Forge;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Выбор релиза (билда):");
                    egui::ComboBox::from_id_source("server_core_version_combo")
                        .selected_text(&app.config.server_core_version)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.config.server_core_version,
                                "Последний стабильный".to_string(),
                                "Последний стабильный",
                            );

                            match app.config.server_core_type {
                                ServerCoreType::Paper => {
                                    let builds = match app.config.minecraft_version.as_str() {
                                        "1.12.2" => vec!["1618", "1617", "1616"],
                                        "1.16.5" => vec!["794", "793", "792"],
                                        "1.20.1" => vec!["196", "195", "194"],
                                        "1.20.4" => vec!["496", "495"],
                                        "1.19.2" => vec!["307", "306"],
                                        "1.18.2" => vec!["388", "387"],
                                        _ => vec![],
                                    };
                                    for build in builds {
                                        ui.selectable_value(
                                            &mut app.config.server_core_version,
                                            build.to_string(),
                                            build,
                                        );
                                    }
                                }
                                ServerCoreType::Forge => {
                                    let versions = match app.config.minecraft_version.as_str() {
                                        "1.12.2" => vec!["14.23.5.2860", "14.23.5.2859"],
                                        "1.16.5" => vec!["36.2.34"],
                                        "1.20.1" => vec!["47.2.0"],
                                        "1.7.10" => vec!["10.13.4.1614"],
                                        _ => vec![],
                                    };
                                    for ver in versions {
                                        ui.selectable_value(
                                            &mut app.config.server_core_version,
                                            ver.to_string(),
                                            ver,
                                        );
                                    }
                                }
                                _ => {}
                            }
                        });
                });

                let filename = match app.config.server_core_type {
                    ServerCoreType::Vanilla => {
                        format!("minecraft_server_{}.jar", app.config.minecraft_version)
                    }
                    ServerCoreType::Paper => format!("paper-{}.jar", app.config.minecraft_version),
                    ServerCoreType::Forge => {
                        format!("forge-server-{}.jar", app.config.minecraft_version)
                    }
                    ServerCoreType::Fabric => {
                        format!("fabric-server-{}.jar", app.config.minecraft_version)
                    }
                    ServerCoreType::NeoForge => {
                        format!("neoforge-server-{}.jar", app.config.minecraft_version)
                    }
                };

                let file_path = format!("{}/{}", SERVER_DIR, filename);
                let file_installed = Path::new(&file_path).exists();
                ui.horizontal(|ui| {
                    if file_installed {
                        ui.colored_label(
                            egui::Color32::LIGHT_GREEN,
                            format!("🟢 Ядро сервера {} установлено", filename),
                        );
                    } else {
                        ui.colored_label(
                            egui::Color32::LIGHT_RED,
                            format!("🔴 Ядро сервера {} отсутствует", filename),
                        );
                    }

                    if ui.button("📥 Скачать серверное ядро").clicked()
                        && !app.is_downloading_server_core
                    {
                        app.is_downloading_server_core = true;
                        download_server_core(
                            ctx.clone(),
                            app.tx.clone(),
                            app.config.server_core_type,
                            app.config.minecraft_version.clone(),
                            app.config.server_core_version.clone(),
                            filename,
                            format!("{:?} Server Core", app.config.server_core_type),
                        );
                    }
                });
            });
        }
        crate::models::ModdingSubTab::Mods => {
            if app.config.mod_loader == ModLoader::Vanilla {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label("⚠️ Модификации (моды) недоступны для Vanilla!");
                    ui.label("Выберите Forge, Fabric или NeoForge в подразделе 'Ядра & Версии'.");
                });
            } else {
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("📦 Загрузка и управление модами (mods/)").strong(),
                    );
                    ui.add_space(4.0);

                    ui.label("Добавить .jar мода по URL:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut app.new_mod_url);
                        if ui.button("➕ Добавить мод").clicked() {
                            let trimmed = app.new_mod_url.trim().to_string();
                            if !trimmed.is_empty()
                                && (trimmed.starts_with("http://")
                                    || trimmed.starts_with("https://"))
                            {
                                let filename = get_filename_from_url(&trimmed);
                                let mods = &mut app.config.mods;
                                if mods.iter().any(|m| m.filename == filename) {
                                    app.status_message = "Мод уже в списке!".to_string();
                                } else {
                                    mods.push(ModConfig {
                                        url: trimmed,
                                        filename: filename.clone(),
                                        allow_update: true,
                                        sha256: None,
                                        enabled: true,
                                    });
                                    app.new_mod_url.clear();
                                    app.save_config();
                                }
                            }
                        }
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.label("Список активных модов (Вкл | Обновление):");
                    let mut needs_save = false;
                    egui::ScrollArea::vertical()
                        .id_source("mods_scroll_area")
                        .max_height(100.0)
                        .show(ui, |ui| {
                            let mods = &mut app.config.mods;
                            let hash_cache = &mut app.hash_cache;
                            if mods.is_empty() {
                                ui.label("Моды отсутствуют.");
                            } else {
                                let mut to_delete = None;
                                for (i, m) in mods.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .checkbox(&mut m.allow_update, "Обновлять")
                                            .on_hover_text(
                                                "Разрешить автоматическое обновление мода по URL",
                                            )
                                            .changed()
                                        {
                                            needs_save = true;
                                        }
                                        let (status_text, _status_color) = get_file_status(
                                            "mods",
                                            hash_cache,
                                            &m.filename,
                                            &m.sha256,
                                            m.enabled,
                                        );
                                        let (icon, icon_color) = match status_text.as_str() {
                                            "Выключен" => ("⚪", egui::Color32::GRAY),
                                            "Отсутствует (Будет скачан)" => {
                                                ("🔴", egui::Color32::from_rgb(230, 100, 100))
                                            }
                                            "Готов (OK)" => {
                                                ("🟢", egui::Color32::from_rgb(100, 230, 100))
                                            }
                                            "Изменен (Будет обновлен)" => {
                                                ("🟡", egui::Color32::from_rgb(230, 180, 50))
                                            }
                                            s if s.starts_with("Установлен") => {
                                                ("🔵", egui::Color32::from_rgb(100, 200, 250))
                                            }
                                            _ => ("🔴", egui::Color32::from_rgb(230, 100, 100)),
                                        };

                                        let m_filename = m.filename.clone();
                                        let m_url = m.url.clone();
                                        let m_sha256 = m.sha256.clone();
                                        let m_enabled = m.enabled;
                                        let m_allow_update = m.allow_update;
                                        let status_text_clone = status_text.clone();

                                        let mod_info_response = ui
                                            .horizontal(|ui| {
                                                ui.label(&m.filename);
                                                ui.colored_label(icon_color, icon);
                                            })
                                            .response;

                                        mod_info_response.on_hover_ui(|ui| {
                                            ui.heading("📦 Детали мода");
                                            ui.add_space(2.0);

                                            ui.horizontal(|ui| {
                                                ui.label("• Файл:");
                                                ui.strong(&m_filename);
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("• Статус:");
                                                ui.colored_label(icon_color, &status_text_clone);
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("• Состояние:");
                                                let (state_str, state_color) = if m_enabled {
                                                    (
                                                        "активен",
                                                        egui::Color32::from_rgb(100, 230, 100),
                                                    )
                                                } else {
                                                    (
                                                        "отключен",
                                                        egui::Color32::from_rgb(230, 100, 100),
                                                    )
                                                };
                                                ui.colored_label(state_color, state_str);
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("• Автообновление:");
                                                let (upd_str, upd_color) = if m_allow_update {
                                                    (
                                                        "разрешено",
                                                        egui::Color32::from_rgb(100, 230, 100),
                                                    )
                                                } else {
                                                    (
                                                        "запрещено",
                                                        egui::Color32::from_rgb(230, 100, 100),
                                                    )
                                                };
                                                ui.colored_label(upd_color, upd_str);
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("• SHA-256:");
                                                ui.monospace(
                                                    m_sha256.as_deref().unwrap_or("не указана"),
                                                );
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("• Ссылка:");
                                                ui.weak(&m_url);
                                            });

                                            ui.add_space(4.0);
                                            ui.separator();
                                            ui.add_space(4.0);

                                            ui.label(
                                                egui::RichText::new("🎨 Легенда статусов:")
                                                    .strong(),
                                            );
                                            ui.horizontal(|ui| {
                                                ui.colored_label(
                                                    egui::Color32::from_rgb(100, 230, 100),
                                                    "🟢",
                                                );
                                                ui.label("— Готов к игре (OK)");
                                            });
                                            ui.horizontal(|ui| {
                                                ui.colored_label(
                                                    egui::Color32::from_rgb(100, 200, 250),
                                                    "🔵",
                                                );
                                                ui.label("— Установлен локально");
                                            });
                                            ui.horizontal(|ui| {
                                                ui.colored_label(
                                                    egui::Color32::from_rgb(230, 180, 50),
                                                    "🟡",
                                                );
                                                ui.label("— Изменен (будет обновлен)");
                                            });
                                            ui.horizontal(|ui| {
                                                ui.colored_label(
                                                    egui::Color32::from_rgb(230, 100, 100),
                                                    "🔴",
                                                );
                                                ui.label("— Отсутствует (будет скачан)");
                                            });
                                            ui.horizontal(|ui| {
                                                ui.colored_label(egui::Color32::GRAY, "⚪");
                                                ui.label("— Отключен");
                                            });
                                        });

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button("❌")
                                                    .on_hover_text("Удалить мод")
                                                    .clicked()
                                                {
                                                    to_delete = Some(i);
                                                }

                                                let (btn_text, btn_color) = if m.enabled {
                                                    ("Вкл", egui::Color32::from_rgb(46, 125, 50))
                                                } else {
                                                    ("Выкл", egui::Color32::from_rgb(198, 40, 40))
                                                };
                                                let toggle_btn = egui::Button::new(
                                                    egui::RichText::new(btn_text)
                                                        .color(egui::Color32::WHITE),
                                                )
                                                .fill(btn_color);
                                                if ui
                                                    .add(toggle_btn)
                                                    .on_hover_text("Включить/выключить мод")
                                                    .clicked()
                                                {
                                                    m.enabled = !m.enabled;
                                                    needs_save = true;
                                                }
                                            },
                                        );
                                    });
                                }
                                if let Some(idx) = to_delete {
                                    let mod_to_remove = &mods[idx];
                                    let mods_dir = Path::new(SERVER_DIR).join("mods");
                                    let jar_path = mods_dir.join(&mod_to_remove.filename);
                                    let disabled_path = mods_dir
                                        .join(format!("{}.disabled", mod_to_remove.filename));
                                    if jar_path.exists() {
                                        let _ = std::fs::remove_file(jar_path);
                                    }
                                    if disabled_path.exists() {
                                        let _ = std::fs::remove_file(disabled_path);
                                    }
                                    mods.remove(idx);
                                    needs_save = true;
                                }
                            }
                        });

                    if needs_save {
                        app.save_config();
                    }

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("🔄 Синхронизировать только моды").clicked()
                            && !app.is_downloading
                        {
                            app.is_downloading = true;
                            sync_mods(ctx.clone(), app.tx.clone(), app.config.mods.clone(), false);
                        }
                        if ui.button("🔍 Обновить из папки mods/").clicked() {
                            scan_local_mods(app);
                        }
                    });
                });
            }
        }
        crate::models::ModdingSubTab::Plugins => {
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new("🧩 Загрузка и управление плагинами (plugins/)").strong(),
                );
                ui.add_space(4.0);

                ui.label("Добавить .jar плагина по URL:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut app.new_plugin_url);
                    if ui.button("➕ Добавить плагин").clicked() {
                        let trimmed = app.new_plugin_url.trim().to_string();
                        if !trimmed.is_empty()
                            && (trimmed.starts_with("http://") || trimmed.starts_with("https://"))
                        {
                            let filename = get_filename_from_url(&trimmed);
                            let plugins = &mut app.config.plugins;
                            if plugins.iter().any(|p| p.filename == filename) {
                                app.status_message = "Плагин уже в списке!".to_string();
                            } else {
                                plugins.push(ModConfig {
                                    url: trimmed,
                                    filename: filename.clone(),
                                    allow_update: true,
                                    sha256: None,
                                    enabled: true,
                                });
                                app.new_plugin_url.clear();
                                app.save_config();
                            }
                        }
                    }
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label("Список установленных плагинов (Вкл | Обновление):");
                let mut needs_save = false;
                egui::ScrollArea::vertical()
                    .id_source("plugins_scroll_area")
                    .max_height(100.0)
                    .show(ui, |ui| {
                        let plugins = &mut app.config.plugins;
                        let hash_cache = &mut app.hash_cache;
                        if plugins.is_empty() {
                            ui.label("Плагины отсутствуют.");
                        } else {
                            let mut to_delete = None;
                            for (i, p) in plugins.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    if ui
                                        .checkbox(&mut p.allow_update, "Обновлять")
                                        .on_hover_text(
                                            "Разрешить автоматическое обновление плагина по URL",
                                        )
                                        .changed()
                                    {
                                        needs_save = true;
                                    }
                                    let (status_text, _status_color) = get_file_status(
                                        "plugins",
                                        hash_cache,
                                        &p.filename,
                                        &p.sha256,
                                        p.enabled,
                                    );
                                    let (icon, icon_color) = match status_text.as_str() {
                                        "Выключен" => ("⚪", egui::Color32::GRAY),
                                        "Отсутствует (Будет скачан)" => {
                                            ("🔴", egui::Color32::from_rgb(230, 100, 100))
                                        }
                                        "Готов (OK)" => {
                                            ("🟢", egui::Color32::from_rgb(100, 230, 100))
                                        }
                                        "Изменен (Будет обновлен)" => {
                                            ("🟡", egui::Color32::from_rgb(230, 180, 50))
                                        }
                                        s if s.starts_with("Установлен") => {
                                            ("🔵", egui::Color32::from_rgb(100, 200, 250))
                                        }
                                        _ => ("🔴", egui::Color32::from_rgb(230, 100, 100)),
                                    };

                                    let p_filename = p.filename.clone();
                                    let p_url = p.url.clone();
                                    let p_sha256 = p.sha256.clone();
                                    let p_enabled = p.enabled;
                                    let p_allow_update = p.allow_update;
                                    let status_text_clone = status_text.clone();

                                    let plugin_info_response = ui
                                        .horizontal(|ui| {
                                            ui.label(&p.filename);
                                            ui.colored_label(icon_color, icon);
                                        })
                                        .response;

                                    plugin_info_response.on_hover_ui(|ui| {
                                        ui.heading("🧩 Детали плагина");
                                        ui.add_space(2.0);

                                        ui.horizontal(|ui| {
                                            ui.label("• Файл:");
                                            ui.strong(&p_filename);
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("• Статус:");
                                            ui.colored_label(icon_color, &status_text_clone);
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("• Состояние:");
                                            let (state_str, state_color) = if p_enabled {
                                                ("активен", egui::Color32::from_rgb(100, 230, 100))
                                            } else {
                                                ("отключен", egui::Color32::from_rgb(230, 100, 100))
                                            };
                                            ui.colored_label(state_color, state_str);
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("• Автообновление:");
                                            let (upd_str, upd_color) = if p_allow_update {
                                                (
                                                    "разрешено",
                                                    egui::Color32::from_rgb(100, 230, 100),
                                                )
                                            } else {
                                                (
                                                    "запрещено",
                                                    egui::Color32::from_rgb(230, 100, 100),
                                                )
                                            };
                                            ui.colored_label(upd_color, upd_str);
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("• SHA-256:");
                                            ui.monospace(
                                                p_sha256.as_deref().unwrap_or("не указана"),
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("• Ссылка:");
                                            ui.weak(&p_url);
                                        });

                                        ui.add_space(4.0);
                                        ui.separator();
                                        ui.add_space(4.0);

                                        ui.label(
                                            egui::RichText::new("🎨 Легенда статусов:").strong(),
                                        );
                                        ui.horizontal(|ui| {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(100, 230, 100),
                                                "🟢",
                                            );
                                            ui.label("— Готов к игре (OK)");
                                        });
                                        ui.horizontal(|ui| {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(100, 200, 250),
                                                "🔵",
                                            );
                                            ui.label("— Установлен локально");
                                        });
                                        ui.horizontal(|ui| {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(230, 180, 50),
                                                "🟡",
                                            );
                                            ui.label("— Изменен (будет обновлен)");
                                        });
                                        ui.horizontal(|ui| {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(230, 100, 100),
                                                "🔴",
                                            );
                                            ui.label("— Отсутствует (будет скачан)");
                                        });
                                        ui.horizontal(|ui| {
                                            ui.colored_label(egui::Color32::GRAY, "⚪");
                                            ui.label("— Отключен");
                                        });
                                    });

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .button("❌")
                                                .on_hover_text("Удалить плагин")
                                                .clicked()
                                            {
                                                to_delete = Some(i);
                                            }

                                            let (btn_text, btn_color) = if p.enabled {
                                                ("Вкл", egui::Color32::from_rgb(46, 125, 50))
                                            } else {
                                                ("Выкл", egui::Color32::from_rgb(198, 40, 40))
                                            };
                                            let toggle_btn = egui::Button::new(
                                                egui::RichText::new(btn_text)
                                                    .color(egui::Color32::WHITE),
                                            )
                                            .fill(btn_color);
                                            if ui
                                                .add(toggle_btn)
                                                .on_hover_text("Включить/выключить плагин")
                                                .clicked()
                                            {
                                                p.enabled = !p.enabled;
                                                needs_save = true;
                                            }
                                        },
                                    );
                                });
                            }
                            if let Some(idx) = to_delete {
                                let plugin_to_remove = &plugins[idx];
                                let plugins_dir = Path::new(SERVER_DIR).join("plugins");
                                let jar_path = plugins_dir.join(&plugin_to_remove.filename);
                                let disabled_path = plugins_dir
                                    .join(format!("{}.disabled", plugin_to_remove.filename));
                                if jar_path.exists() {
                                    let _ = std::fs::remove_file(jar_path);
                                }
                                if disabled_path.exists() {
                                    let _ = std::fs::remove_file(disabled_path);
                                }
                                plugins.remove(idx);
                                needs_save = true;
                            }
                        }
                    });

                if needs_save {
                    app.save_config();
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("🔄 Синхронизировать плагины").clicked()
                        && !app.is_downloading_plugins
                    {
                        app.is_downloading_plugins = true;
                        sync_plugins(ctx.clone(), app.tx.clone(), app.config.plugins.clone());
                    }
                    if ui.button("🔍 Обновить из папки plugins/").clicked() {
                        scan_local_plugins(app);
                    }
                    ui.label("💡 Плагины используются серверами (Spigot/Paper).");
                });
            });
        }
        crate::models::ModdingSubTab::Dependencies => {
            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("☕ Среда Java JRE").strong());
                        ui.add_space(4.0);

                        let req_java =
                            get_java_version_for_minecraft(&app.config.minecraft_version);
                        ui.label(format!(
                            "Необходима Java {} для MC {}",
                            req_java, app.config.minecraft_version
                        ));

                        for &java_v in &[8, 17, 21] {
                            let java_path = format!("{}/jre{}/bin/java", SERVER_DIR, java_v);
                            let installed = Path::new(&java_path).exists();
                            ui.horizontal(|ui| {
                                if installed {
                                    ui.colored_label(
                                        egui::Color32::LIGHT_GREEN,
                                        format!("🟢 Java {} установлена", java_v),
                                    );
                                } else {
                                    ui.colored_label(
                                        egui::Color32::LIGHT_RED,
                                        format!("🔴 Java {} отсутствует", java_v),
                                    );
                                    if ui.button(format!("📥 Скачать Java {}", java_v)).clicked()
                                        && !app.is_downloading_java
                                    {
                                        app.is_downloading_java = true;
                                        download_jre(ctx.clone(), app.tx.clone(), java_v);
                                    }
                                }
                            });
                        }
                    });
                });

                columns[1].vertical(|ui| {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("📚 Библиотеки игры").strong());
                        ui.add_space(4.0);
                        ui.label(format!(
                            "Версия: {}, Ядро: {:?}",
                            app.config.minecraft_version, app.config.mod_loader
                        ));
                        if ui.button("📥 Скачать/Обновить библиотеки").clicked()
                            && !app.is_downloading_libs
                        {
                            app.is_downloading_libs = true;
                            download_client_libs(
                                ctx.clone(),
                                app.tx.clone(),
                                app.config.minecraft_version.clone(),
                                app.config.mod_loader,
                            );
                        }
                    });
                });
            });
        }
    }
}

pub fn scan_local_mods(app: &mut LauncherApp) {
    let mods_dir = std::path::Path::new(SERVER_DIR).join("mods");
    if !mods_dir.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(mods_dir) {
        let mut added_any = false;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy();
                    if ext_str == "jar" || ext_str == "disabled" {
                        let mut filename = path.file_name().unwrap().to_string_lossy().into_owned();
                        let mut enabled = true;
                        if ext_str == "disabled" {
                            if let Some(stem) = path.file_stem() {
                                let stem_path = std::path::Path::new(&stem);
                                if stem_path.extension().is_some_and(|e| e == "jar") {
                                    filename = stem.to_string_lossy().into_owned();
                                    enabled = false;
                                }
                            }
                        }

                        let exists = app.config.mods.iter().any(|m| m.filename == filename);
                        if !exists {
                            app.config.mods.push(crate::models::ModConfig {
                                url: "".to_string(),
                                filename,
                                allow_update: false,
                                sha256: None,
                                enabled,
                            });
                            added_any = true;
                        }
                    }
                }
            }
        }
        if added_any {
            app.save_config();
        }
    }
}

pub fn scan_local_plugins(app: &mut LauncherApp) {
    let plugins_dir = std::path::Path::new(SERVER_DIR).join("plugins");
    if !plugins_dir.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(plugins_dir) {
        let mut added_any = false;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy();
                    if ext_str == "jar" || ext_str == "disabled" {
                        let mut filename = path.file_name().unwrap().to_string_lossy().into_owned();
                        let mut enabled = true;
                        if ext_str == "disabled" {
                            if let Some(stem) = path.file_stem() {
                                let stem_path = std::path::Path::new(&stem);
                                if stem_path.extension().is_some_and(|e| e == "jar") {
                                    filename = stem.to_string_lossy().into_owned();
                                    enabled = false;
                                }
                            }
                        }

                        let exists = app.config.plugins.iter().any(|p| p.filename == filename);
                        if !exists {
                            app.config.plugins.push(crate::models::ModConfig {
                                url: "".to_string(),
                                filename,
                                allow_update: false,
                                sha256: None,
                                enabled,
                            });
                            added_any = true;
                        }
                    }
                }
            }
        }
        if added_any {
            app.save_config();
        }
    }
}
