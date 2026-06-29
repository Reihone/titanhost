use crate::ui::LauncherApp;
use eframe::egui;

/// Render the Server tab panel (IP configuration, server configs automation generation)
pub fn draw_server_tab(app: &mut LauncherApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("📡 Конфигурация подключения к серверу").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("IP адрес сервера:");
            let edit_ip = ui.text_edit_singleline(&mut app.config.server_ip);
            if edit_ip.changed() {
                *app.server_address.lock().unwrap() =
                    (app.config.server_ip.clone(), app.config.server_port);
            }
        });

        ui.horizontal(|ui| {
            ui.label("Порт сервера:");
            let mut port_str = app.config.server_port.to_string();
            let edit_port = ui.text_edit_singleline(&mut port_str);
            if edit_port.changed() {
                if let Ok(parsed) = port_str.parse::<u16>() {
                    app.config.server_port = parsed;
                    *app.server_address.lock().unwrap() =
                        (app.config.server_ip.clone(), app.config.server_port);
                }
            }
        });
    });

    ui.add_space(10.0);
    ui.group(|ui| {
        ui.label(egui::RichText::new("⚙️ Игровые настройки (server.properties)").strong());
        ui.add_space(6.0);

        ui.columns(2, |cols| {
            cols[0].vertical(|ui| {
                ui.checkbox(
                    &mut app.config.online_mode,
                    "🔒 Проверка лицензии (online-mode)",
                )
                .on_hover_text("Если выключено, то игроки с пиратских лаунчеров смогут войти.");

                ui.checkbox(&mut app.config.white_list, "📝 Белый список (whitelist)")
                    .on_hover_text("Только игроки из whitelist.json смогут зайти на сервер.");

                ui.checkbox(&mut app.config.pvp, "⚔️ Включить PvP")
                    .on_hover_text("Разрешает игрокам наносить урон друг другу.");

                ui.checkbox(&mut app.config.spawn_monsters, "🧟 Спавн монстров")
                    .on_hover_text("Включает появление враждебных мобов в мире.");

                ui.checkbox(
                    &mut app.config.allow_flight,
                    "✈️ Разрешить полеты (allow-flight)",
                )
                .on_hover_text("Разрешить игрокам использовать флай-хаки или летать в выживании.");

                ui.checkbox(&mut app.config.hardcore, "💀 Режим Хардкор (hardcore)")
                    .on_hover_text("Включает хардкорный режим (одна жизнь).");

                ui.checkbox(
                    &mut app.config.generate_structures,
                    "🏰 Генерация структур (structures)",
                )
                .on_hover_text("Генерировать крепости, деревни и шахты в новом мире.");

                ui.checkbox(
                    &mut app.config.enable_command_block,
                    "🧱 Командные блоки (command-blocks)",
                )
                .on_hover_text("Включить выполнение команд командными блоками.");
            });

            cols[1].vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Сложность:");
                    egui::ComboBox::from_id_source("difficulty_combo")
                        .selected_text(match app.config.difficulty.as_str() {
                            "peaceful" => "Peaceful (Мирная)",
                            "easy" => "Easy (Легкая)",
                            "normal" => "Normal (Нормальная)",
                            "hard" => "Hard (Сложная)",
                            _ => "Normal (Нормальная)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.config.difficulty,
                                "peaceful".to_string(),
                                "Peaceful (Мирная)",
                            );
                            ui.selectable_value(
                                &mut app.config.difficulty,
                                "easy".to_string(),
                                "Easy (Легкая)",
                            );
                            ui.selectable_value(
                                &mut app.config.difficulty,
                                "normal".to_string(),
                                "Normal (Нормальная)",
                            );
                            ui.selectable_value(
                                &mut app.config.difficulty,
                                "hard".to_string(),
                                "Hard (Сложная)",
                            );
                        });
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Режим игры:");
                    egui::ComboBox::from_id_source("gamemode_combo")
                        .selected_text(match app.config.gamemode.as_str() {
                            "survival" => "Выживание (Survival)",
                            "creative" => "Творческий (Creative)",
                            "adventure" => "Приключение (Adventure)",
                            "spectator" => "Наблюдатель (Spectator)",
                            _ => "Выживание (Survival)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.config.gamemode,
                                "survival".to_string(),
                                "Выживание (Survival)",
                            );
                            ui.selectable_value(
                                &mut app.config.gamemode,
                                "creative".to_string(),
                                "Творческий (Creative)",
                            );
                            ui.selectable_value(
                                &mut app.config.gamemode,
                                "adventure".to_string(),
                                "Приключение (Adventure)",
                            );
                            ui.selectable_value(
                                &mut app.config.gamemode,
                                "spectator".to_string(),
                                "Наблюдатель (Spectator)",
                            );
                        });
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Макс. игроков:");
                    ui.add(egui::Slider::new(&mut app.config.max_players, 1..=200));
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Прорисовка (view-distance):");
                    ui.add(
                        egui::Slider::new(&mut app.config.view_distance, 3..=32).suffix(" чанков"),
                    );
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Защита спавна (блоки):");
                    ui.add(egui::Slider::new(&mut app.config.spawn_protection, 0..=100));
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Сид мира (seed):");
                    ui.text_edit_singleline(&mut app.config.level_seed);
                });
            });
        });
    });

    ui.add_space(10.0);
    ui.group(|ui| {
        ui.label(egui::RichText::new("🔄 Поведение при сбоях (Crash Recovery)").strong());
        ui.add_space(4.0);

        ui.checkbox(
            &mut app.config.auto_restart_on_crash,
            "🚀 Авторестарт при падении (Crash Detection)",
        )
        .on_hover_text(
            "Автоматически перезапускает сервер, если игровой процесс завершился с кодом ошибки.",
        );
    });

    ui.add_space(10.0);
    ui.group(|ui| {
        ui.label(egui::RichText::new("🌐 Локальная сеть ZeroTier").strong());
        ui.add_space(4.0);
        ui.label("Используйте ZeroTier, чтобы ваши друзья могли подключиться к серверу через виртуальную локальную сеть без настройки роутера.");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(250, 180, 50), "⚠️ В разработке (временно недоступно)");
        });
        ui.add_space(6.0);

        ui.add_enabled_ui(false, |ui| {
            let token_opt = app.zerotier_token.clone();
            let status_opt = app.zerotier_status.clone();
            let networks_opt = app.zerotier_networks.clone();

            if let Some(token) = token_opt {
                match status_opt {
                    Some(Ok(status)) => {
                        ui.horizontal(|ui| {
                            ui.label("Статус демона:");
                            if status.online {
                                ui.colored_label(egui::Color32::from_rgb(100, 230, 100), "🟢 В сети");
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(230, 100, 100), "🔴 Офлайн");
                            }
                            
                            ui.add_space(10.0);
                            ui.label("Версия:");
                            ui.label(&status.version);
                        });

                        ui.horizontal(|ui| {
                            ui.label("ID узла (Node ID):");
                            ui.label(egui::RichText::new(&status.address).monospace().strong());
                            if ui.small_button("📋 Копировать").clicked() {
                                ui.output_mut(|o| o.copied_text = status.address.clone());
                                app.status_message = "Node ID скопирован!".to_string();
                            }
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        ui.label(egui::RichText::new("Подключенные сети:").strong());
                        if let Some(Ok(networks)) = networks_opt {
                            if networks.is_empty() {
                                ui.label("Нет подключенных сетей ZeroTier.");
                            } else {
                                for net in networks {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Сеть:");
                                            ui.label(egui::RichText::new(&net.id).monospace().strong());
                                            if !net.name.is_empty() {
                                                ui.label(format!("({})", net.name));
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let token_clone = token.clone();
                                                let net_id = net.id.clone();
                                                let tx = app.tx.clone();
                                                if ui.button("❌ Выйти").clicked() {
                                                    tokio::spawn(async move {
                                                        let _ = tx.send(format!("LOG: [СИСТЕМА] Выход из сети ZeroTier {}...", net_id));
                                                        match crate::core::zerotier::leave_network(&token_clone, &net_id).await {
                                                            Ok(_) => {
                                                                let _ = tx.send(format!("LOG: [СИСТЕМА] Вышли из сети ZeroTier {}", net_id));
                                                            }
                                                            Err(e) => {
                                                                let _ = tx.send(format!("LOG: [ОШИБКА] Не удалось выйти из сети {}: {}", net_id, e));
                                                            }
                                                        }
                                                    });
                                                }
                                            });
                                        });

                                        ui.horizontal(|ui| {
                                            ui.label("Статус:");
                                            let status_color = match net.status.as_str() {
                                                "OK" => egui::Color32::from_rgb(100, 230, 100),
                                                "ACCESS_DENIED" => egui::Color32::from_rgb(230, 100, 100),
                                                _ => egui::Color32::from_rgb(250, 210, 100),
                                            };
                                            ui.colored_label(status_color, &net.status);

                                            ui.add_space(10.0);
                                            ui.label("Интерфейс:");
                                            ui.label(&net.port_device_name);
                                        });

                                        if !net.assigned_addresses.is_empty() {
                                            ui.horizontal(|ui| {
                                                ui.label("IP-адреса:");
                                                for addr in &net.assigned_addresses {
                                                    let clean_ip = addr.split('/').next().unwrap_or(addr);
                                                    ui.label(egui::RichText::new(clean_ip).monospace().strong());
                                                    
                                                    if ui.small_button("📌 Привязать IP").clicked() {
                                                        app.config.server_ip = clean_ip.to_string();
                                                        *app.server_address.lock().unwrap() = (app.config.server_ip.clone(), app.config.server_port);
                                                        app.status_message = format!("IP привязан: {}", clean_ip);
                                                        app.log_messages.push(format!("[СИСТЕМА] IP сервера изменен на ZeroTier IP: {}", clean_ip));
                                                        app.save_config();
                                                    }
                                                }
                                            });
                                        } else {
                                            ui.colored_label(egui::Color32::from_rgb(230, 100, 100), "⚠️ Нет IP. Проверьте авторизацию в панели ZeroTier.");
                                        }
                                    });
                                    ui.add_space(4.0);
                                }
                            }
                        } else if let Some(Err(e)) = networks_opt {
                            ui.colored_label(egui::Color32::from_rgb(230, 100, 100), format!("Ошибка получения сетей: {}", e));
                        } else {
                            ui.label("Загрузка списка сетей...");
                        }

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Подключить сеть (Network ID):");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.zerotier_join_input)
                                    .hint_text("16-значный ID сети")
                                    .desired_width(150.0)
                            );
                            
                            let can_join = app.zerotier_join_input.trim().len() == 16;
                            if ui.add_enabled(can_join, egui::Button::new("🔗 Подключиться")).clicked() {
                                let net_id = app.zerotier_join_input.trim().to_string();
                                let token_clone = token.clone();
                                let tx = app.tx.clone();
                                tokio::spawn(async move {
                                    let _ = tx.send(format!("LOG: [СИСТЕМА] Подключение к сети ZeroTier {}...", net_id));
                                    match crate::core::zerotier::join_network(&token_clone, &net_id).await {
                                        Ok(_) => {
                                            let _ = tx.send(format!("LOG: [СИСТЕМА] Запрос на подключение к {} отправлен. Ожидайте авторизации в панели управления ZeroTier (если сеть приватная).", net_id));
                                        }
                                        Err(e) => {
                                            let _ = tx.send(format!("LOG: [ОШИБКА] Не удалось подключиться к сети {}: {}", net_id, e));
                                        }
                                    }
                                });
                                app.zerotier_join_input.clear();
                            }
                        });
                    }
                    Some(Err(e)) => {
                        ui.colored_label(egui::Color32::from_rgb(230, 100, 100), format!("Ошибка ZeroTier API: {}", e));
                        ui.label("Убедитесь, что сервис ZeroTier запущен на вашем компьютере.");
                    }
                    None => {
                        ui.label("Подключение к API ZeroTier...");
                    }
                }
            } else {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(230, 100, 100), "🔒 Доступ к ZeroTier не авторизован");
                });
                ui.label("Для чтения статуса и автоматического управления сетями необходимо авторизовать лаунчер. Это потребует подтверждения прав администратора.");
                ui.add_space(4.0);
                if ui.button("🔑 Авторизовать доступ через PolicyKit").clicked() {
                    let tx = app.tx.clone();
                    std::thread::spawn(move || {
                        let _ = tx.send("LOG: [СИСТЕМА] Запрос прав администратора для чтения authtoken.secret...".to_string());
                        match crate::core::zerotier::fetch_authtoken_via_pkexec() {
                            Ok(_) => {
                                let _ = tx.send("LOG: [СИСТЕМА] ZeroTier успешно авторизован! Получен токен.".to_string());
                            }
                            Err(e) => {
                                let _ = tx.send(format!("LOG: [ОШИБКА] Ошибка авторизации ZeroTier: {}", e));
                            }
                        }
                    });
                }
            }
        });
    });

    ui.add_space(10.0);
    ui.group(|ui| {
        ui.label(egui::RichText::new("🛠 Автоматизация сборки сервера").strong());
        ui.add_space(4.0);
        ui.label("Сгенерировать файлы конфигурации (eula.txt, server.properties) и скрипты запуска (run.bat / run.sh) на основе параметров, указанных выше.");
        
        ui.add_space(4.0);
        if ui.button("⚡ Сгенерировать конфигурации сервера").clicked() {
            match crate::core::process::generate_server_files(&app.config) {
                Ok(_) => {
                    app.status_message = "Файлы сервера сгенерированы!".to_string();
                    app.log_messages.push("[СИСТЕМА] Успешно сгенерированы: eula.txt, server.properties, run.bat/run.sh".to_string());
                }
                Err(e) => {
                    app.status_message = "Ошибка генерации файлов!".to_string();
                    app.log_messages.push(format!("[ОШИБКА] Не удалось сгенерировать файлы сервера: {}", e));
                }
            }
        }
    });
}
