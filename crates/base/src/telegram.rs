//! Port of `Telegram.cs`: notify a Telegram user through the LaserGRBL service endpoint.
//!
//! The bot code is stored by C# DPAPI-encrypted (`Tools/DataProtector.cs`, `Protector`); here it is
//! stored as plain text in the settings file (written with 0600 permissions). The endpoint and service
//! key come from [`crate::url_manager`] and are `None` by default, so nothing is sent unless configured.

use crate::url_manager::urls;
use std::time::Duration;

/// `Telegram.NotifyEvent(message)`: use the configured code when notifications are enabled.
///
/// `message`: text to send. `guid`: anonymous install id (C# `UsageStats.GetID()`).
/// Settings: `TelegramNotification.Enabled`, `TelegramNotification.Code` (plain).
pub fn notify_event(message: &str, guid: &str) {
    if crate::settings::get("TelegramNotification.Enabled", false) {
        let code: String = crate::settings::get("TelegramNotification.Code", String::new());
        notify_event_code(&code, message, guid);
    }
}

/// `Telegram.NotifyEvent(usercode, message)`: async POST (form fields servicekey, id, message, guid).
///
/// `usercode`: 10 char Telegram code (trimmed; other lengths are ignored). `message`: text.
/// `guid`: install id. Returns immediately; the request runs on a thread with 5 s timeout.
pub fn notify_event_code(usercode: &str, message: &str, guid: &str) {
    let u = urls();
    let (Some(handler), Some(key)) = (u.telegram_handler, u.telegram_service_key) else { return };
    let usercode = usercode.trim();
    if usercode.chars().count() != 10 {
        return;
    }
    let fields = [
        ("servicekey".to_string(), key),
        ("id".to_string(), usercode.to_string()),
        ("message".to_string(), message.to_string()),
        ("guid".to_string(), guid.to_string()),
    ];
    let _ = std::thread::Builder::new().name("Telegram".into()).spawn(move || internal_notify(&handler, &fields));
}

/// `InternalNotifyEvent`: blocking POST; errors ignored like C#.
///
/// `url`: handler URL. `fields`: form fields.
fn internal_notify(url: &str, fields: &[(String, String)]) {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(5))).build().into();
    let _ = agent.post(url).send_form(fields.iter().map(|(k, v)| (k.as_str(), v.as_str())));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telegram_flow() {
        // default urls are None: must return without any network
        notify_event_code("0123456789", "hi", "guid");
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut buf = vec![0u8; 4096];
            let mut got = Vec::new();
            // read until body present
            loop {
                let n = s.read(&mut buf).unwrap();
                got.extend_from_slice(&buf[..n]);
                let t = String::from_utf8_lossy(&got).to_string();
                if t.contains("guid=") || n == 0 {
                    break;
                }
            }
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");
            String::from_utf8_lossy(&got).to_string()
        });
        crate::url_manager::set_urls(crate::url_manager::UrlManager {
            telegram_handler: Some(format!("http://127.0.0.1:{port}/tg")),
            telegram_service_key: Some("KEY".into()),
            ..Default::default()
        });
        notify_event_code(" 0123456789 ", "hello world", "G1");
        let req = h.join().unwrap();
        crate::url_manager::set_urls(Default::default());
        assert!(req.starts_with("POST /tg"), "{req}");
        assert!(req.contains("servicekey=KEY") && req.contains("id=0123456789") && req.contains("message=hello+world") || req.contains("message=hello%20world"), "{req}");
        assert!(req.contains("guid=G1"), "{req}");
    }
}
