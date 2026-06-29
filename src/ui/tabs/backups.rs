use crate::ui::LauncherApp;
use eframe::egui;
use std::path::Path;
use std::process::Command;

pub fn draw_backups_tab(app: &mut LauncherApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.heading("💾 Резервные копии (Бэкапы)");
        ui.add_space(4.0);
        ui.label("Создавайте полные резервные копии сервера (мир, моды, плагины, настройки) перед обновлениями или важными изменениями.");
        
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let is_running = app.active_process.is_some();
            if is_running {
                ui.add_enabled(false, egui::Button::new("➕ Создать бэкап"))
                    .on_hover_text("Нельзя делать бэкап при запущенном сервере!");
                ui.colored_label(egui::Color32::from_rgb(230, 100, 100), "⚠️ Остановите сервер для создания/восстановления резервных копий.");
            } else {
                if ui.button("➕ Создать резервную копию").clicked() {
                    let secs = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs();
                    let date_str = Command::new("date")
                        .arg("+%Y%m%d_%H%M%S")
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| secs.to_string());
                    
                    let backups_dir = Path::new(crate::core::SERVER_DIR).join("backups");
                    let filepath = backups_dir.join(format!("backup_{}.zip", date_str));
                    
                    app.add_system_log(format!("[БЭКАП] Запуск создания бэкапа: backup_{}.zip...", date_str));
                    app.status_message = "Создание резервной копии...".to_string();
                    
                    match crate::core::backup::create_backup_archive(crate::core::SERVER_DIR, filepath.to_str().unwrap_or("")) {
                        Ok(_) => {
                            app.add_system_log(format!("[БЭКАП] Успешно создана резервная копия: backup_{}.zip", date_str));
                            app.status_message = "Резервная копия создана!".to_string();
                            app.backups_list = crate::core::backup::list_backups(crate::core::SERVER_DIR);
                        }
                        Err(e) => {
                            app.add_system_log(format!("[БЭКАП] Ошибка создания резервной копии: {}", e));
                            app.status_message = "Ошибка бэкапа!".to_string();
                        }
                    }
                }
            }
        });
    });

    ui.add_space(10.0);

    ui.group(|ui| {
        ui.label(egui::RichText::new("📋 Список доступных резервных копий").strong());
        ui.add_space(6.0);

        let backups = app.backups_list.clone();
        if backups.is_empty() {
            ui.colored_label(egui::Color32::GRAY, "Нет доступных резервных копий.");
        } else {
            egui::ScrollArea::vertical().id_source("backups_scroll").max_height(250.0).show(ui, |ui| {
                let mut needs_refresh = false;
                let is_running = app.active_process.is_some();
                
                for backup in backups {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("📦").size(18.0));
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&backup.filename).strong());
                            ui.horizontal(|ui| {
                                ui.colored_label(egui::Color32::GRAY, format!("Дата: {}", backup.date_str));
                                ui.add_space(10.0);
                                ui.colored_label(egui::Color32::LIGHT_BLUE, format!("Размер: {:.2} МБ", backup.size_mb));
                            });
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Delete button
                            if ui.button("🗑 Удалить").clicked() {
                                let filepath = Path::new(crate::core::SERVER_DIR).join("backups").join(&backup.filename);
                                if filepath.exists() {
                                    let _ = std::fs::remove_file(filepath);
                                    app.add_system_log(format!("[БЭКАП] Удален файл резервной копии: {}", backup.filename));
                                    needs_refresh = true;
                                }
                            }

                            // Restore button
                            if is_running {
                                ui.add_enabled(false, egui::Button::new("🔄 Восстановить"))
                                    .on_hover_text("Нельзя восстанавливать бэкап при запущенном сервере!");
                            } else {
                                if ui.button("🔄 Восстановить").clicked() {
                                    let filepath = Path::new(crate::core::SERVER_DIR).join("backups").join(&backup.filename);
                                    app.add_system_log(format!("[БЭКАП] Начало восстановления из бэкапа {}...", backup.filename));
                                    app.status_message = "Восстановление бэкапа...".to_string();

                                    match crate::core::backup::restore_backup_archive(crate::core::SERVER_DIR, filepath.to_str().unwrap_or("")) {
                                        Ok(_) => {
                                            app.add_system_log(format!("[БЭКАП] Успешно восстановлен сервер из: {}", backup.filename));
                                            app.status_message = "Сервер восстановлен!".to_string();
                                            // Refresh mods & plugins local scan
                                            crate::ui::tabs::modding::scan_local_mods(app);
                                            crate::ui::tabs::modding::scan_local_plugins(app);
                                        }
                                        Err(e) => {
                                            app.add_system_log(format!("[БЭКАП] Ошибка восстановления: {}", e));
                                            app.status_message = "Ошибка восстановления!".to_string();
                                        }
                                    }
                                }
                            }
                        });
                    });
                    ui.separator();
                }
                
                if needs_refresh {
                    app.backups_list = crate::core::backup::list_backups(crate::core::SERVER_DIR);
                }
            });
        }
    });
}
