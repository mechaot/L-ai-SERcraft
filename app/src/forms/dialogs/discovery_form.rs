// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `WiFiDiscovery/DiscoveryForm.cs` + `DiscoveryForm.Designer.cs`: find the engraver on the LAN.
//!
//! The scan itself is `lg_services::wifi_discovery::scan_ip` on a worker thread.

use eframe::egui::{self, Ui, Vec2};
use lasergrbl_core::GrblCore;
use lg_services::wifi_discovery::{self as wd, DiscoveryProtocol, ScanResult};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::common::{ModalEvent, button_en, list_view, modal, trf};
use crate::ui::icons::icons;
use crate::ui::widgets::NumericUpDown;

/// Result of the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryOutcome {
    /// Closed without choosing (C# returned null) or serial protocol selected (nothing to return).
    None,
    /// Chosen address: `ip:port` (telnet) or `ws://ip:port/` (websocket).
    Address(String),
}

/// Messages of the scan thread.
enum Msg {
    Result(ScanResult),
    Progress(i32, i32, i32),
    Ended,
}

/// The dialog.
pub struct DiscoveryForm {
    protocol: DiscoveryProtocol,
    port: f64,
    results: Vec<ScanResult>,
    selected: Option<usize>,
    progress: String,
    rx: Option<crossbeam_channel::Receiver<Msg>>,
    cancel: Arc<AtomicBool>,
    stopping: bool,
    returned: Option<String>,
    id: u64,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Header of the connection column (`UdPort_ValueChanged`).
///
/// `port`: spinner value. Returns e.g. `"Connection (Port 23)"`.
pub fn connection_header(port: u16) -> String {
    format!("Connection (Port {port})")
}

impl DiscoveryForm {
    /// Create the dialog (C# ctor): initial port from the configured protocol.
    ///
    /// `core`: engine (board type decides the telnet port).
    pub fn open(core: &Arc<GrblCore>) -> Self {
        let proto: String = lg_base::settings::get("ComWrapper Protocol", "UsbSerial".to_string());
        Self::new(DiscoveryProtocol::from_setting_name(&proto), core.is_longer_board())
    }

    /// Same with explicit values.
    ///
    /// `protocol`: current protocol. `is_longer_board`: Longer board flag.
    pub fn new(protocol: DiscoveryProtocol, is_longer_board: bool) -> Self {
        DiscoveryForm {
            protocol,
            port: wd::default_port(protocol, is_longer_board) as f64,
            results: Vec::new(),
            selected: None,
            progress: trf("WiFiDiscovery.DiscoveryForm.LblProgress.Text", "Progress:"),
            rx: None,
            cancel: Arc::new(AtomicBool::new(false)),
            stopping: false,
            returned: None,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Port as number.
    fn port_u16(&self) -> u16 {
        self.port.clamp(1.0, 65535.0) as u16
    }

    /// Scan running.
    pub fn scanning(&self) -> bool {
        self.rx.is_some()
    }

    /// `StartScan`.
    fn start_scan(&mut self, ctx: &egui::Context) {
        if self.scanning() {
            return;
        }
        self.results.clear();
        self.selected = None;
        self.stopping = false;
        self.cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = crossbeam_channel::unbounded();
        let cancel = self.cancel.clone();
        let port = self.port_u16();
        let ctx = ctx.clone();
        let _ = std::thread::Builder::new().name("WiFiScan".into()).spawn(move || {
            let (t1, t2, c1, c2) = (tx.clone(), tx.clone(), ctx.clone(), ctx.clone());
            let on_result = move |r: &ScanResult| {
                // rows refresh when a result changes (ResultItem.OnUpdate)
                let c = c1.clone();
                r.subscribe(move || c.request_repaint());
                let _ = t1.send(Msg::Result(r.clone()));
                c1.request_repaint();
            };
            let on_progress = move |ph: i32, n: i32, t: i32| {
                let _ = t2.send(Msg::Progress(ph, n, t));
                c2.request_repaint();
            };
            if let Err(e) = wd::scan_ip(&on_result, &on_progress, &cancel, port, None) {
                lg_base::logger::log_message("WiFiDiscovery", &e);
            }
            let _ = tx.send(Msg::Ended);
            ctx.request_repaint();
        });
        self.rx = Some(rx);
    }

    /// `StopScan`: set the cancel flag.
    fn stop_scan(&mut self) {
        self.stopping = true;
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// `ReturnItem` for result `idx`. Returns the outcome when the protocol yields an address.
    fn return_item(&mut self, idx: usize) -> Option<DiscoveryOutcome> {
        let r = self.results.get(idx)?;
        self.returned = wd::result_address(self.protocol, r);
        Some(self.returned.clone().map(DiscoveryOutcome::Address).unwrap_or(DiscoveryOutcome::None))
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<DiscoveryOutcome> {
        self.pump();
        if self.scanning() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        let title = trf("WiFiDiscovery.DiscoveryForm.$this.Text", "WiFi Discovery");
        let id = ("discovery", self.id);
        match modal(ctx, id, &title, 551.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(out)) => {
                self.stop_scan();
                Some(out)
            }
            Some(ModalEvent::Closed) => {
                // DiscoveryForm_FormClosing
                self.stop_scan();
                Some(DiscoveryOutcome::None)
            }
            None => None,
        }
    }

    /// Drain scan thread messages.
    fn pump(&mut self) {
        let Some(rx) = &self.rx else { return };
        let mut ended = false;
        while let Ok(m) = rx.try_recv() {
            match m {
                Msg::Result(r) => self.results.push(r),
                Msg::Progress(ph, n, t) => {
                    if let Some(s) = wd::progress_label(ph, n, t) {
                        self.progress = s;
                    }
                }
                Msg::Ended => ended = true,
            }
        }
        if ended {
            // OnScanEnd
            self.progress = if self.cancel.load(Ordering::SeqCst) { "Scan aborted".into() } else { "Scan finished".into() };
            self.rx = None;
            self.stopping = false;
        }
    }

    /// Form body. Returns the outcome to close with.
    fn body(&mut self, ui: &mut Ui) -> Option<DiscoveryOutcome> {
        let t = |c: &str, d: &str| trf(&format!("WiFiDiscovery.DiscoveryForm.{c}.Text"), d);
        ui.horizontal_top(|ui| {
            ui.add(icons().image(ui.ctx(), "mdi-wifi", 64.0));
            ui.add(egui::Label::new(t("LblDescription", "")).wrap());
        });
        ui.add_space(4.0);
        // ListView 539x172, column widths from the designer (ping keeps the 60px default)
        let heads = [t("ChIP", "IP Address"), t("ChPing", "Ping"), t("ChHostName", "Host Name"), t("ChMAC", "MAC Address"), connection_header(self.port_u16())];
        let widths = [100.0, 60.0, 120.0, 100.0, 130.0];
        let rows: Vec<Vec<String>> = self
            .results
            .iter()
            .map(|r| vec![r.ip().to_string(), r.ping().unwrap_or_default(), r.host_name().unwrap_or_default(), r.mac().unwrap_or_default(), r.telnet().unwrap_or_default()])
            .collect();
        let mut sel = self.selected;
        let lv = list_view(ui, "wifi-grid", &heads, &widths, &rows, &mut sel, 172.0);
        let mut out = None;
        // LV_MouseDoubleClick
        if let Some(i) = lv.double_clicked {
            out = self.return_item(i);
        }
        self.selected = sel;
        ui.label(&self.progress);
        ui.add_space(4.0);
        let scanning = self.scanning();
        let ctx = ui.ctx().clone();
        let mut scan = false;
        let mut stop = false;
        let mut connect = false;
        ui.horizontal(|ui| {
            ui.label(t("label1", "TCP Port:"));
            ui.add_enabled_ui(!scanning, |ui| {
                ui.add(NumericUpDown::new(&mut self.port).range(1.0, 65535.0).width(70.0).id(egui::Id::unique("wifi-port")));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let sz = Vec2::new(80.0, 34.0);
                // BtnScan / BtnStop share the slot
                if scanning {
                    stop = button_en(ui, !self.stopping, &t("BtnStop", "Stop"), None, sz);
                } else {
                    scan = button_en(ui, true, &t("BtnScan", "Scan"), None, sz);
                }
                connect = button_en(ui, self.selected.is_some(), &t("BtnConnect", "Connect"), None, Vec2::new(86.0, 34.0));
            });
        });
        if scan {
            self.start_scan(&ctx);
        }
        if stop {
            self.progress = "Abort scan...".into();
            self.stop_scan();
        }
        if connect {
            if let Some(i) = self.selected {
                out = out.or_else(|| self.return_item(i));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_ports() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        assert_eq!(DiscoveryForm::new(DiscoveryProtocol::Telnet, false).port_u16(), 23);
        assert_eq!(DiscoveryForm::new(DiscoveryProtocol::Telnet, true).port_u16(), 8847);
        assert_eq!(DiscoveryForm::new(DiscoveryProtocol::LaserWebEsp8266, false).port_u16(), 81);
        assert_eq!(DiscoveryForm::new(DiscoveryProtocol::UsbSerial, false).port_u16(), 666);
    }

    #[test]
    fn return_item_formats_address() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let mut f = DiscoveryForm::new(DiscoveryProtocol::Telnet, false);
        f.results.push(ScanResult::new("192.168.1.20".parse().unwrap(), 23));
        assert_eq!(f.return_item(0), Some(DiscoveryOutcome::Address("192.168.1.20:23".into())));
        f.protocol = DiscoveryProtocol::LaserWebEsp8266;
        assert_eq!(f.return_item(0), Some(DiscoveryOutcome::Address("ws://192.168.1.20:23/".into())));
        f.protocol = DiscoveryProtocol::UsbSerial;
        assert_eq!(f.return_item(0), Some(DiscoveryOutcome::None));
        assert_eq!(f.return_item(5), None);
        assert_eq!(connection_header(81), "Connection (Port 81)");
    }
}
