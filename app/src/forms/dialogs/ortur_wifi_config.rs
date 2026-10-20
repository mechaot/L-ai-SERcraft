//! Ports `WiFiConfigurator/OrturWiFiConfig.cs` + `OrturWiFiConfig.Designer.cs`: program SSID / password
//! into an Ortur or Longer engraver and wait for its IP.

use eframe::egui::{self, Color32, RichText, TextEdit, Ui, Vec2};
use lasergrbl_core::GrblCore;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::common::{ModalEvent, button_en, modal, open_link, trf};
use crate::ui::dialog::{DialogResult, MessageBox, MessageBoxButtons, MessageBoxIcon};
use crate::ui::icons::icons;
use crate::ui::widgets::ImageButton;

/// Seconds to wait for the machine IP before giving up (`TimWait_Tick`).
const WAIT_LIMIT_S: f64 = 10.0;

/// Result of the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WifiConfigOutcome {
    /// Closed without a successful configuration.
    Cancelled,
    /// IP detected and the user agreed to disconnect USB and connect via WiFi (`DialogResult.OK`).
    ConnectViaWifi { ip: String },
}

/// `FindSSID` on Windows: first `SSID` line (not `BSSID`) of `netsh wlan show interfaces`.
///
/// `output`: command output. Returns the SSID or empty.
pub fn parse_netsh_ssid(output: &str) -> String {
    let line = output.lines().filter(|l| !l.trim().is_empty()).find(|l| l.contains("SSID") && !l.contains("BSSID"));
    match line {
        None => String::new(),
        // split on ':' removing empty entries, take the second part
        Some(l) => l.split(':').filter(|p| !p.is_empty()).nth(1).map(|s| s.trim_start().trim_end_matches('\r').to_owned()).unwrap_or_default(),
    }
}

/// `nmcli -t -f active,ssid dev wifi` (Linux): SSID of the active network.
///
/// `output`: command output (`yes:MySsid` lines). Returns the SSID or empty.
pub fn parse_nmcli_active(output: &str) -> String {
    output.lines().find_map(|l| l.strip_prefix("yes:")).map(|s| s.replace("\\:", ":")).unwrap_or_default()
}

/// SSID of the network the PC is on (C# ran `netsh`; Linux tries `nmcli`, then `iwgetid`).
///
/// Returns the SSID or empty.
pub fn current_ssid() -> String {
    let run = |prog: &str, args: &[&str]| std::process::Command::new(prog).args(args).stdin(std::process::Stdio::null()).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
    if cfg!(windows) {
        return run("netsh.exe", &["wlan", "show", "interfaces"]).map(|o| parse_netsh_ssid(&o)).unwrap_or_default();
    }
    if let Some(s) = run("nmcli", &["-t", "-f", "active,ssid", "dev", "wifi"]).map(|o| parse_nmcli_active(&o)).filter(|s| !s.is_empty()) {
        return s;
    }
    run("iwgetid", &["-r"]).map(|s| s.trim().to_owned()).unwrap_or_default()
}

/// `OnTextChanges`: Write needs a non-blank SSID and password.
///
/// `ssid`,`pwd`: text boxes. Returns true when Write is enabled.
pub fn write_enabled(ssid: &str, pwd: &str) -> bool {
    !ssid.trim().is_empty() && !pwd.trim().is_empty()
}

/// Text of the wait label: base text plus the elapsed seconds after the first second.
///
/// `base`: translated message. `elapsed`: seconds since Write. Returns the label text.
pub fn wait_text(base: &str, elapsed: f64) -> String {
    if elapsed > 1.0 { format!("{base} ({})", elapsed as i32) } else { base.to_owned() }
}

/// The dialog.
pub struct OrturWiFiConfig {
    core: Arc<GrblCore>,
    ssid: String,
    pwd: String,
    waiting: Option<Instant>,
    done: Option<Arc<std::sync::atomic::AtomicBool>>,
    ask: Option<(String, MessageBox)>,
    msg: Option<MessageBox>,
    reveal: bool,
    id: u64,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl OrturWiFiConfig {
    /// Create the dialog (C# ctor): SSID from `$74` or the system, password from `$75`.
    ///
    /// `core`: engine used for `write_wifi_config` and the detected IP.
    pub fn open(core: &Arc<GrblCore>) -> Self {
        let conf = GrblCore::configuration();
        let ssid = match conf.wifi_ssid() {
            Some(s) if !s.trim().is_empty() => s,
            _ => current_ssid(),
        };
        OrturWiFiConfig {
            core: core.clone(),
            ssid,
            pwd: conf.wifi_pwd().unwrap_or_default(),
            waiting: None,
            done: None,
            ask: None,
            msg: None,
            reveal: false,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// `WaitMode(v)`.
    fn wait_mode(&mut self, on: bool) {
        self.waiting = on.then(Instant::now);
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<WifiConfigOutcome> {
        // result boxes of TimWait_Tick
        if let Some((ip, mb)) = &mut self.ask {
            if let Some(r) = mb.show(ctx) {
                let ip = ip.clone();
                self.ask = None;
                if r == DialogResult::Ok {
                    return Some(WifiConfigOutcome::ConnectViaWifi { ip });
                }
            }
            return None;
        }
        if let Some(mb) = &mut self.msg {
            if mb.show(ctx).is_some() {
                self.msg = None;
            }
            return None;
        }
        self.tick(ctx);
        let title = trf("WiFiConfigurator.OrturWiFiConfig.$this.Text", "WiFi Configuration");
        let id = ("ortur-wifi", self.id);
        match modal(ctx, id, &title, 499.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(())) => None,
            Some(ModalEvent::Closed) => {
                // OrturWiFiConfig_FormClosing
                self.wait_mode(false);
                Some(WifiConfigOutcome::Cancelled)
            }
            None => None,
        }
    }

    /// `TimWait_Tick`.
    fn tick(&mut self, ctx: &egui::Context) {
        let Some(t0) = self.waiting else { return };
        ctx.request_repaint_after(Duration::from_millis(250));
        // the blocking write must have finished before the wait is judged
        if self.done.as_ref().is_some_and(|d| !d.load(std::sync::atomic::Ordering::SeqCst)) {
            return;
        }
        if let Some(ip) = self.core.detected_ip() {
            self.wait_mode(false);
            let text = format!("Detected machine IP {ip}\r\nDo you want to disconnect USB and connect via WiFi?");
            self.ask = Some((ip, MessageBox::new(text, "Configuration success!").buttons(MessageBoxButtons::OkCancel).icon(MessageBoxIcon::Information)));
        } else if t0.elapsed().as_secs_f64() >= WAIT_LIMIT_S {
            self.wait_mode(false);
            self.msg = Some(
                MessageBox::new(
                    "Unable to establish a connection: machine not connected to the WiFi network or IP address not assigned correctly. Please retry or proceed with a manual configuration.",
                    "Error",
                )
                .icon(MessageBoxIcon::Warning),
            );
        }
    }

    /// `BtnWrite_Click`: wait mode + blocking core write on a worker.
    fn write(&mut self, ctx: &egui::Context) {
        self.wait_mode(true);
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.done = Some(flag.clone());
        let (core, ssid, pwd, ctx) = (self.core.clone(), self.ssid.clone(), self.pwd.clone(), ctx.clone());
        let _ = std::thread::Builder::new().name("WiFiWrite".into()).spawn(move || {
            core.write_wifi_config(&ssid, &pwd);
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            ctx.request_repaint();
        });
    }

    /// Form body. Returns Some(()) when nothing special (kept for the modal helper).
    fn body(&mut self, ui: &mut Ui) -> Option<()> {
        let t = |c: &str, d: &str| trf(&format!("WiFiConfigurator.OrturWiFiConfig.{c}.Text"), d);
        let wait = self.waiting.is_some();
        ui.horizontal_top(|ui| {
            ui.add(icons().image(ui.ctx(), "mdi-wifi", 64.0));
            ui.add(egui::Label::new(t("LblDescription", "In order to activate WiFi, SSID name and WiFi password must be programmed on the machine. Please enter these details.")).wrap());
        });
        ui.add_space(6.0);
        let mut reveal = false;
        ui.add_enabled_ui(!wait, |ui| {
            egui::Grid::new(("wifi-grid", self.id)).num_columns(3).spacing([8.0, 6.0]).show(ui, |ui| {
                ui.label(t("LblSSID", "SSID (network name)"));
                ui.add_sized([276.0, 20.0], TextEdit::singleline(&mut self.ssid));
                ui.label("");
                ui.end_row();
                ui.label(t("LblPassword", "Password"));
                // UseSystemPasswordChar until PbRevealPwd is held down
                let shown = self.reveal;
                ui.add_sized([276.0, 20.0], TextEdit::singleline(&mut self.pwd).password(!shown));
                let r = ui.add(ImageButton::new("mdi-eye").size(Vec2::splat(22.0)));
                reveal = r.is_pointer_button_down_on();
                ui.end_row();
            });
        });
        self.reveal = reveal;
        if wait {
            let elapsed = self.waiting.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
            ui.label(RichText::new(wait_text(&t("LblWaitConnection", "Engraver is trying to connect to WiFi, please wait ..."), elapsed)).color(Color32::RED));
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            // BtnHelp is Visible=false in the designer and never shown by the code
            let _ = open_link;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if button_en(ui, !wait && write_enabled(&self.ssid, &self.pwd), &t("BtnWrite", "Write"), None, Vec2::new(107.0, 42.0)) {
                    let ctx = ui.ctx().clone();
                    self.write(&ctx);
                }
            });
        });
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssid_parsers() {
        let netsh = "There is 1 interface on the system:\r\n\r\n    Name                   : Wi-Fi\r\n    SSID                   : HomeNet\r\n    BSSID                  : aa:bb:cc:dd:ee:ff\r\n";
        assert_eq!(parse_netsh_ssid(netsh), "HomeNet");
        assert_eq!(parse_netsh_ssid("nothing here"), "");
        assert_eq!(parse_nmcli_active("no:Other\nyes:My\\:Net\n"), "My:Net");
        assert_eq!(parse_nmcli_active("no:Other\n"), "");
    }

    #[test]
    fn write_rules_and_wait_text() {
        assert!(write_enabled("a", "b"));
        assert!(!write_enabled(" ", "b"));
        assert!(!write_enabled("a", ""));
        assert_eq!(wait_text("wait", 0.5), "wait");
        assert_eq!(wait_text("wait", 1.0), "wait");
        assert_eq!(wait_text("wait", 3.9), "wait (3)");
    }
}
