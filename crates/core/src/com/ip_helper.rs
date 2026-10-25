// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `IPHelper` (in `ComWrapper/Telnet.cs`): parse `host[:port]`, `[v6]:port` and bare IPv6.

use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};

/// `IPHelper.Parse(endpointstring)` with default port -1.
///
/// `s`: text like `192.168.1.5:23`, `host:23`, `[::1]:23`. Returns the endpoint; Err when empty, port missing
/// or invalid, or the host cannot be resolved (C# throws `ArgumentException`/`FormatException`).
pub fn parse_endpoint(s: &str) -> io::Result<SocketAddr> {
    parse_endpoint_default(s, -1)
}

fn bad(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg)
}

/// `IPHelper.Parse(endpointstring, defaultport)`.
///
/// `s`: endpoint text. `defaultport`: port used when the text has none (-1 = none, error).
pub fn parse_endpoint_default(s: &str, defaultport: i32) -> io::Result<SocketAddr> {
    if s.trim().is_empty() {
        return Err(bad("Endpoint descriptor may not be empty.".into()));
    }
    if defaultport != -1 && !(0..=65535).contains(&defaultport) {
        return Err(bad(format!("Invalid default port '{defaultport}'")));
    }
    let values: Vec<&str> = s.split(':').collect();
    let ip: IpAddr;
    let port: i32;
    if values.len() <= 2 {
        // ipv4 or hostname
        port = if values.len() == 1 { defaultport } else { get_port(values[1])? };
        ip = match values[0].parse::<IpAddr>() {
            Ok(a) => a,
            Err(_) => get_ip_from_host(values[0])?,
        };
    } else if values[0].starts_with('[') && values[values.len() - 2].ends_with(']') {
        // [a:b:c]:d
        let addr = values[..values.len() - 1].join(":");
        let inner = addr.trim_start_matches('[').trim_end_matches(']');
        ip = inner.parse::<IpAddr>().map_err(|_| bad(format!("Invalid endpoint ipaddress '{s}'")))?;
        port = get_port(values[values.len() - 1])?;
    } else {
        // [a:b:c] or a:b:c
        let inner = s.trim_start_matches('[').trim_end_matches(']');
        ip = inner.parse::<IpAddr>().map_err(|_| bad(format!("Invalid endpoint ipaddress '{s}'")))?;
        port = defaultport;
    }
    if port == -1 {
        return Err(bad(format!("No port specified: '{s}'")));
    }
    Ok(SocketAddr::new(ip, port as u16))
}

/// `getPort`: 0..65535.
fn get_port(p: &str) -> io::Result<i32> {
    match lg_dotnet::parse_int(p) {
        Some(v) if (0..=65535).contains(&v) => Ok(v),
        _ => Err(bad(format!("Invalid end point port '{p}'"))),
    }
}

/// `getIPfromHost`: first resolved address.
fn get_ip_from_host(h: &str) -> io::Result<IpAddr> {
    (h, 0u16)
        .to_socket_addrs()
        .ok()
        .and_then(|mut it| it.next())
        .map(|a| a.ip())
        .ok_or_else(|| bad(format!("Host not found: {h}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        assert_eq!(parse_endpoint("192.168.1.5:23").unwrap().to_string(), "192.168.1.5:23");
        assert_eq!(parse_endpoint("127.0.0.1:8080").unwrap().port(), 8080);
        assert_eq!(parse_endpoint("[::1]:23").unwrap().to_string(), "[::1]:23");
        assert_eq!(parse_endpoint_default("::1", 23).unwrap().port(), 23);
        assert_eq!(parse_endpoint_default("10.0.0.1", 23).unwrap().to_string(), "10.0.0.1:23");
        assert_eq!(parse_endpoint("localhost:23").unwrap().port(), 23);
    }

    #[test]
    fn errors() {
        assert!(parse_endpoint("").is_err());
        assert!(parse_endpoint("   ").is_err());
        assert!(parse_endpoint("10.0.0.1").is_err());
        assert!(parse_endpoint("10.0.0.1:99999").is_err());
        assert!(parse_endpoint("10.0.0.1:abc").is_err());
        assert!(parse_endpoint_default("10.0.0.1", 70000).is_err());
    }
}
