#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::{Color32, FontId, RichText};
use egui_extras::{Column, TableBuilder};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };
    eframe::run_native("mymachi", options, Box::new(|_cc| Box::<MyApp>::default()))
}

#[derive(Clone, Debug)]
struct Server {
    port: u16,
    running: bool,
    name: String,
}

struct MyApp {
    service_ip: String,
    input_port: String,
    input_name: String,
    added_servers: Vec<Server>,
    input_port_correct: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            service_ip: "0.0.0.0".to_owned(),
            input_port: "25565".to_owned(),
            input_name: "Minecraft".to_owned(),
            added_servers: Vec::new(),
            input_port_correct: true,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let service_ip_label = ui.label("service IP: ");
                ui.text_edit_singleline(&mut self.service_ip)
                    .labelled_by(service_ip_label.id);
            });
            ui.vertical(|ui| {
                ui.heading("Добавить сервер");
                ui.horizontal(|ui| {
                    let port_label = ui.label("port: ");
                    if ui
                        .text_edit_singleline(&mut self.input_port)
                        .labelled_by(port_label.id)
                        .changed()
                    {
                        match self.input_port.parse::<u16>() {
                            Ok(port) => {
                                self.input_port_correct = true;
                                port
                            }
                            Err(_) => {
                                self.input_port_correct = false;
                                0
                            }
                        };
                    }
                    let name_label = ui.label("Имя");
                    ui.text_edit_singleline(&mut self.input_name)
                        .labelled_by(name_label.id);
                    if ui
                        .add_enabled(
                            self.input_port_correct,
                            egui::Button::new(if !self.input_port_correct {
                                RichText::new("Неверный формат").color(Color32::from_rgb(255, 0, 0))
                            } else {
                                RichText::new("Добавить")
                            }),
                        )
                        .clicked()
                    {
                        self.added_servers.push(Server {
                            port: self.input_port.parse::<u16>().unwrap(),
                            running: true,
                            name: self.input_name.clone(),
                        });
                    }
                });
                let total_server_count = self.added_servers.len();
                let mut running_servers_count = 0;
                TableBuilder::new(ui)
                    .auto_shrink([false, true])
                    .column(Column::auto().resizable(true).at_least(50.0))
                    .column(Column::auto().resizable(true).at_least(200.0))
                    .column(Column::auto().resizable(true).at_least(100.0))
                    .column(Column::auto().resizable(true).at_least(100.0))
                    .column(Column::auto().resizable(true).at_least(70.0))
                    .header(30.0, |mut header| {
                        let headers = vec!["Номер", "Имя", "Порт", "", ""];
                        for h in headers {
                            header.col(|ui| {
                                ui.heading(h.clone());
                            });
                        }
                    })
                    .body(|mut body| {
                        for (index, added_server) in
                            self.added_servers.clone().iter_mut().enumerate()
                        {
                            body.row(30.0, |mut row| {
                                row.col(|ui| {
                                    ui.label((index + 1).to_string());
                                });
                                row.col(|ui| {
                                    ui.label(added_server.name.clone());
                                });
                                row.col(|ui| {
                                    ui.label(added_server.port.clone().to_string());
                                });
                                if added_server.running {
                                    running_servers_count += 1;
                                }
                                let status = if added_server.running {
                                    RichText::new("Включен").color(Color32::from_rgb(100, 255, 100))
                                } else {
                                    RichText::new("Выключен")
                                        .color(Color32::from_rgb(200, 200, 200))
                                };
                                if index < self.added_servers.len() {
                                    row.col(|ui| {
                                        ui.checkbox(&mut self.added_servers[index].running, status);
                                    });
                                    row.col(|ui| {
                                        if ui.button("Удалить").clicked() {
                                            self.added_servers.remove(index);
                                        }
                                    });
                                }
                            });
                        }
                    });
                if total_server_count > 0 {
                    ui.label(
                        RichText::new(format!("Всего серверов: {}", total_server_count))
                            .font(FontId::proportional(20.0)),
                    );
                    if running_servers_count == total_server_count {
                        ui.label(
                            RichText::new("Все серверы включены")
                                .font(FontId::proportional(20.0))
                                .color(Color32::from_rgb(100, 255, 100)),
                        );
                    } else if running_servers_count == 0 {
                        ui.label(
                            RichText::new("Все серверы выключены")
                                .font(FontId::proportional(20.0))
                                .color(Color32::from_rgb(255, 100, 100)),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!(
                                "Включенных серверов: {}",
                                running_servers_count
                            ))
                            .font(FontId::proportional(20.0))
                            .color(Color32::from_rgb(100, 255, 100)),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Выключенных серверов: {}",
                                total_server_count - running_servers_count
                            ))
                            .font(FontId::proportional(20.0))
                            .color(Color32::from_rgb(255, 100, 100)),
                        );
                    }
                }

                // for (index, added_server) in self.added_servers.clone().iter_mut().enumerate() {
                //     ui.horizontal(|ui| {
                //         ui.label(
                //             RichText::new(format!(
                //                 "{}\t{}:{}",
                //                 index + 1,
                //                 added_server.name,
                //                 added_server.port
                //             ))
                //             .color(if added_server.running {
                //                 Color32::from_rgb(100, 255, 100)
                //             } else {
                //                 Color32::from_rgb(200, 200, 200)
                //             }),
                //         );
                //         let status = if added_server.running {
                //             "Включен"
                //         } else {
                //             "Выключен"f
                //         };
                //         if index < self.added_servers.len() {
                //             let c = ui.checkbox(&mut self.added_servers[index].running, status);
                //             if ui.button("Удалить").clicked() {
                //                 self.added_servers.remove(index);
                //             }
                //         }
                //     });
                // }
            });
        });
    }
}