use crate::core::process::MINECRAFT_VERSIONS;
use crate::models::{LauncherConfig, ModLoader};
use crate::ui::LauncherApp;
use eframe::egui;

/// Render the main launcher tab (profile settings, versions, RAM sliders)
pub fn draw_launch_tab(app: &mut LauncherApp, ui: &mut egui::Ui, _ctx: &egui::Context) {
    ui.columns(2, |columns| {
        // Left Column: Profile selector and version selectors
        columns[0].vertical(|ui| {
            ui.group(|ui| {
                ui.label(egui::RichText::new("⚙️ Выбор сборки").strong());
                ui.add_space(4.0);

                if app.show_new_profile_input {
                    ui.horizontal(|ui| {
                        ui.label("Имя профиля:");
                        ui.text_edit_singleline(&mut app.new_profile_name);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("➕ Создать").clicked() {
                            let name = app.new_profile_name.trim().to_string();
                            if !name.is_empty() {
                                let default_config = LauncherConfig::default();
                                app.profiles_data
                                    .profiles
                                    .insert(name.clone(), default_config);
                                app.selected_profile = name;
                                app.config = app
                                    .profiles_data
                                    .profiles
                                    .get(&app.selected_profile)
                                    .cloned()
                                    .unwrap_or_default();
                                *app.server_address.lock().unwrap() =
                                    (app.config.server_ip.clone(), app.config.server_port);
                                app.new_profile_name.clear();
                                app.show_new_profile_input = false;
                                app.save_config();
                            }
                        }
                        if ui.button("Отмена").clicked() {
                            app.show_new_profile_input = false;
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Игровой профиль:");
                        egui::ComboBox::from_id_source("profile_combo")
                            .selected_text(&app.selected_profile)
                            .show_ui(ui, |ui| {
                                let mut keys: Vec<String> =
                                    app.profiles_data.profiles.keys().cloned().collect();
                                keys.sort();
                                for key in keys {
                                    if ui
                                        .selectable_value(
                                            &mut app.selected_profile,
                                            key.clone(),
                                            &key,
                                        )
                                        .clicked()
                                    {
                                        app.config = app
                                            .profiles_data
                                            .profiles
                                            .get(&app.selected_profile)
                                            .cloned()
                                            .unwrap_or_default();
                                        *app.server_address.lock().unwrap() =
                                            (app.config.server_ip.clone(), app.config.server_port);
                                        app.save_config();
                                    }
                                }
                            });

                        if ui.button("➕ Новый").clicked() {
                            app.show_new_profile_input = true;
                        }
                    });

                    // Option to delete selected profile if there's more than one
                    if app.profiles_data.profiles.len() > 1 {
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            if ui.button("🗑 Удалить текущий профиль").clicked()
                            {
                                app.profiles_data.profiles.remove(&app.selected_profile);
                                if let Some(first_key) = app.profiles_data.profiles.keys().next() {
                                    app.selected_profile = first_key.clone();
                                }
                                app.config = app
                                    .profiles_data
                                    .profiles
                                    .get(&app.selected_profile)
                                    .cloned()
                                    .unwrap_or_default();
                                *app.server_address.lock().unwrap() =
                                    (app.config.server_ip.clone(), app.config.server_port);
                                app.save_config();
                            }
                        });
                    }
                }

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Версия игры:");
                    egui::ComboBox::from_id_source("version_combo")
                        .selected_text(&app.config.minecraft_version)
                        .show_ui(ui, |ui| {
                            for &ver in MINECRAFT_VERSIONS {
                                if ui
                                    .selectable_value(
                                        &mut app.config.minecraft_version,
                                        ver.to_string(),
                                        ver,
                                    )
                                    .clicked()
                                {
                                    app.save_config();
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Ядро запуска:");
                    let core_name = match app.config.mod_loader {
                        ModLoader::Vanilla => "Vanilla (Чистая)",
                        ModLoader::Forge => "Forge (Моды)",
                        ModLoader::Fabric => "Fabric (Моды)",
                        ModLoader::NeoForge => "NeoForge (Моды)",
                    };
                    ui.colored_label(egui::Color32::LIGHT_GREEN, core_name);
                });
            });
        });

        // Right Column: RAM allotment and JVM settings
        columns[1].vertical(|ui| {
            ui.group(|ui| {
                ui.label(egui::RichText::new("🧠 Память и Оптимизация").strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Выделить ОЗУ:");
                    ui.add(
                        egui::Slider::new(
                            &mut app.config.max_ram_gb,
                            1..=app.total_system_ram.max(16),
                        )
                        .suffix(" ГБ"),
                    );
                });

                ui.horizontal(|ui| {
                    if ui.button("🛠 Умный пресет").clicked() {
                        app.apply_auto_ram_preset();
                    }
                });

                ui.add_space(4.0);
                ui.checkbox(
                    &mut app.config.aikars_flags,
                    "Оптимизация Java (Aikar's Flags)",
                );
            });
        });
    });
}
