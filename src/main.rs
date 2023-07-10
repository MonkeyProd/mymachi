#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::{Color32, FontId, RichText, Window};
use egui_extras::{Column, TableBuilder};
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::thread::JoinHandle;
use tokio::sync::{mpsc::UnboundedSender, watch::Receiver};

struct Network {
    /// Handle to the network thread.
    handle: JoinHandle<()>,
    /// Unbounded sender (of messages) to the network thread.
    submit: UnboundedSender<Message>,
}

#[derive(Debug, Clone)]
pub struct Server {
    port: u16,
    running: bool,
    name: String,
}

struct Address {
    ip: Ipv4Addr,
    port: u16,
}

pub type Message = SendType;

pub enum SendType {
    SendServiceServer(String),
    AddClientServer(Server),
}

fn main() -> std::io::Result<()> {
    let ip = "0.0.0.0".parse::<Ipv4Addr>().unwrap();
    let port = "15151".parse::<u16>().unwrap();
    let addr = "0.0.0.0:11311".parse::<SocketAddr>().unwrap();
    let std_sock = std::net::UdpSocket::bind(addr)?;
    std_sock.set_nonblocking(true)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .thread_name("network")
        .enable_io()
        .build()?;
    let udp = {
        let _guard = runtime.enter();
        tokio::net::UdpSocket::from_std(std_sock)?
    };
    use tokio::sync::{mpsc, watch};
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<Message>();
    let (log_tx, log_rx) = watch::channel(String::new());
    let handle = std::thread::spawn(move || {
        runtime.block_on(async move {
               let mut buf = [0; 64];
               let mut clientServers: Vec<Server> = Vec::new();
               loop {
                   tokio::select! {
                       biased;
                       input_res = msg_rx.recv() => {
                           let Some(input) = input_res else {
                               break;
                           };
                           match input {
                               SendType::SendServiceServer(mes) => { udp.send_to(&mes.into_bytes().into_boxed_slice(), (ip, port)).await.expect("cannot send message to socket");},
                               SendType::AddClientServer(server) => {
                                clientServers.push(server);
                               },
                           };

                       }
                       recv_res = udp.recv_from(&mut buf) => {
                           let (count, remote_addr) = recv_res.expect("cannot receive from socket");
                           if let Ok(parsed) = core::str::from_utf8(&buf[..count]) {
                               log_tx.send_modify(|log| {
                                   use core::fmt::Write;
                                   log.write_fmt(format_args!("[{remote_addr}]: {parsed}\n")).expect("cannot append message to buffer");
                                   log.write_fmt(format_args!("Current servers: {:?}\n", clientServers)).expect("cannot append message to buffer");
                               });
                           }
                       }
                   }
               }
           })
    });

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "mymachi",
        options,
        Box::new(|_cc| Box::new(MyApp::new(handle, msg_tx, log_rx))),
    );
    Ok(())
}

struct MyApp {
    service_ip: String,
    input_port: String,
    input_name: String,
    added_servers: Vec<Server>,
    input_port_correct: bool,
    network: Option<Network>,
    log: Receiver<String>,
    show_server_response_window: bool,
}

impl MyApp {
    pub fn new(
        handle: JoinHandle<()>,
        submit: UnboundedSender<Message>,
        log: Receiver<String>,
    ) -> Self {
        Self {
            service_ip: "0.0.0.0".to_owned(),
            input_port: "25565".to_owned(),
            input_name: "Minecraft".to_owned(),
            added_servers: Vec::new(),
            input_port_correct: true,
            network: Some(Network { handle, submit }),
            show_server_response_window: false,
            log,
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
                ui.checkbox(
                    &mut self.show_server_response_window,
                    "Server response window",
                );
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
                            running: false,
                            name: self.input_name.clone(),
                        });
                        self.network
                            .as_ref()
                            .unwrap()
                            .submit
                            .send(SendType::AddClientServer(
                                self.added_servers.last().unwrap().clone(),
                            ))
                            .expect("receiver closed");
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
                                        if ui
                                            .checkbox(
                                                &mut self.added_servers[index].running,
                                                status,
                                            )
                                            .changed()
                                        {
                                            self.network
                                                .as_ref()
                                                .unwrap()
                                                .submit
                                                .send(SendType::SendServiceServer(
                                                    format!(
                                                        "{} is {}",
                                                        self.added_servers[index].name,
                                                        self.added_servers[index].running
                                                    )
                                                    .to_string(),
                                                ))
                                                .expect("receiver closed");
                                        }
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
                Window::new("Server response")
                    .open(&mut self.show_server_response_window)
                    .vscroll(true)
                    .show(ctx, |ui| {
                        ui.set_min_height(300.0);
                        ui.set_min_width(300.0);
                        ui.label(RichText::new("Server response:").size(15.0));
                        ui.label(self.log.borrow().to_string());
                    });
            });
        });
    }
}
