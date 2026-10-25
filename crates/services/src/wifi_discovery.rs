// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `WiFiDiscovery/IPAddressHelper.cs` and the non-UI parts of `DiscoveryForm.cs`.
//!
//! No new dependencies: interfaces come from `ip -o -4 addr` / `ip -4 route` / `/proc/net/route`
//! (Linux) or `ipconfig` (Windows); ICMP ping runs the system `ping` (falls back to a TCP probe
//! when it cannot be started); MAC comes from `/proc/net/arp` (`arp -a` on Windows) instead of
//! `SendARP`; host names from `getent hosts` (Windows: not available, stays empty).
//! Deviation: pings run on a worker pool instead of the C# 16-per-100ms `SendAsync` pacing.

use std::net::{Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// C# `AVAIL`.
const AVAIL: &str = "Available";
/// Ping / connect timeout (5000 ms in C#).
const TIMEOUT: Duration = Duration::from_secs(5);

// ---------------------------------------------------------------- byte helpers (UInt*Helper)

/// C# `UInt16Helper.ReverseBytes`.
///
/// `value`: input. Returns the byte swapped value.
pub fn reverse_bytes_u16(value: u16) -> u16 {
    value.swap_bytes()
}

/// C# `UInt32Helper.ReverseBytes`.
///
/// `value`: input. Returns the byte swapped value.
pub fn reverse_bytes_u32(value: u32) -> u32 {
    value.swap_bytes()
}

/// C# `UInt64Helper.ReverseBytes`.
///
/// `value`: input. Returns the byte swapped value.
pub fn reverse_bytes_u64(value: u64) -> u64 {
    value.swap_bytes()
}

/// C# `UInt32Helper.ToIPAddress`: `new IPAddress(BitConverter.GetBytes(ip))` (little endian host).
///
/// `ip`: address word as produced by [`to_uint32`]. Returns the address.
pub fn to_ip_address(ip: u32) -> Ipv4Addr {
    Ipv4Addr::from(ip.to_le_bytes())
}

/// C# `IPAddress.ToUInt32`: `BitConverter.ToUInt32(bytes, 0)` (first octet = low byte).
///
/// `ip`: address. Returns the word.
pub fn to_uint32(ip: Ipv4Addr) -> u32 {
    u32::from_le_bytes(ip.octets())
}

/// C# `GetBroadcastAddress`.
///
/// `ip`: host address. `mask`: subnet mask. Returns `ip | ~mask`.
pub fn get_broadcast_address(ip: Ipv4Addr, mask: Ipv4Addr) -> Ipv4Addr {
    let (i, m) = (ip.octets(), mask.octets());
    Ipv4Addr::new(i[0] | !m[0], i[1] | !m[1], i[2] | !m[2], i[3] | !m[3])
}

/// C# `GetNetworkAddress`.
///
/// `ip`: host address. `mask`: subnet mask. Returns `ip & mask`.
pub fn get_network_address(ip: Ipv4Addr, mask: Ipv4Addr) -> Ipv4Addr {
    let (i, m) = (ip.octets(), mask.octets());
    Ipv4Addr::new(i[0] & m[0], i[1] & m[1], i[2] & m[2], i[3] & m[3])
}

/// C# `IsInSameSubnet`.
///
/// `address2`,`address`: hosts. `mask`: subnet mask. Returns true when both share the network.
pub fn is_in_same_subnet(address2: Ipv4Addr, address: Ipv4Addr, mask: Ipv4Addr) -> bool {
    get_network_address(address, mask) == get_network_address(address2, mask)
}

// ---------------------------------------------------------------- interfaces

/// One IPv4 unicast address of an interface (C# `UnicastIPAddressInformation` + adapter facts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetInterface {
    pub name: String,
    pub ip: Ipv4Addr,
    pub mask: Ipv4Addr,
    /// Adapter has a default gateway (`GatewayAddresses.FirstOrDefault() != null`).
    pub has_gateway: bool,
    /// `OperationalStatus == Up`.
    pub up: bool,
}

/// Prefix length to mask.
///
/// `prefix`: 0..=32. Returns the mask.
fn prefix_to_mask(prefix: u32) -> Ipv4Addr {
    let bits: u32 = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix.min(32)) };
    Ipv4Addr::from(bits)
}

/// Parse `ip -o -4 addr show` output.
///
/// `text`: command output. Returns `(name, ip, mask)` per `inet` entry.
pub fn parse_ip_addr(text: &str) -> Vec<(String, Ipv4Addr, Ipv4Addr)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        // "2: eth0    inet 192.168.1.5/24 brd ..."
        let Some(p) = f.iter().position(|x| *x == "inet") else { continue };
        let (Some(name), Some(cidr)) = (f.get(1), f.get(p + 1)) else { continue };
        let name = name.trim_end_matches(':').split('@').next().unwrap_or("").to_string();
        let Some((ip, pre)) = cidr.split_once('/') else { continue };
        if let (Ok(ip), Ok(pre)) = (ip.parse::<Ipv4Addr>(), pre.parse::<u32>()) {
            out.push((name, ip, prefix_to_mask(pre)));
        }
    }
    out
}

/// Parse `ip -4 route show default` output.
///
/// `text`: command output. Returns the device names that carry a default gateway.
pub fn parse_default_route_devices(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.starts_with("default"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            f.iter().position(|x| *x == "dev").and_then(|i| f.get(i + 1)).map(|s| s.to_string())
        })
        .collect()
}

/// Parse `/proc/net/route`.
///
/// `text`: file content. Returns interface names with a default route (destination 0, gateway set).
pub fn parse_proc_route(text: &str) -> Vec<String> {
    text.lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            (f.len() > 2 && f[1] == "00000000" && f[2] != "00000000").then(|| f[0].to_string())
        })
        .collect()
}

/// Parse Windows `ipconfig` output (language independent: values are recognised by shape).
///
/// `text`: command output. Per adapter block the first IPv4 value is the address, the second the
/// mask, the third the gateway. Returns the interfaces.
pub fn parse_ipconfig(text: &str) -> Vec<NetInterface> {
    let mut out = Vec::new();
    let mut name = String::new();
    let mut vals: Vec<Ipv4Addr> = Vec::new();
    let flush = |name: &str, vals: &mut Vec<Ipv4Addr>, out: &mut Vec<NetInterface>| {
        if vals.len() >= 2 {
            out.push(NetInterface { name: name.to_string(), ip: vals[0], mask: vals[1], has_gateway: vals.len() >= 3, up: true });
        }
        vals.clear();
    };
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        // adapter header: not indented, ends with ':'
        if !line.starts_with(' ') && line.trim_end().ends_with(':') {
            flush(&name, &mut vals, &mut out);
            name = line.trim().trim_end_matches(':').to_string();
            continue;
        }
        if let Some((_, v)) = line.split_once(" : ").or_else(|| line.split_once(": ")) {
            let v = v.trim().split('(').next().unwrap_or("").trim();
            if let Ok(ip) = v.parse::<Ipv4Addr>() {
                vals.push(ip);
            }
        }
    }
    flush(&name, &mut vals, &mut out);
    out
}

/// Run a command and return stdout.
///
/// `prog`,`args`: command line. Returns None when it cannot run or fails.
fn run_text(prog: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(prog).args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

/// C# `NetworkInterface.GetAllNetworkInterfaces()` (IPv4 unicast only).
///
/// Returns all interfaces with an IPv4 address; empty when the platform tools are missing.
pub fn list_interfaces() -> Vec<NetInterface> {
    if cfg!(windows) {
        return run_text("ipconfig", &[]).map(|t| parse_ipconfig(&t)).unwrap_or_default();
    }
    let addrs = run_text("ip", &["-o", "-4", "addr", "show"]).map(|t| parse_ip_addr(&t)).unwrap_or_default();
    let mut gw = run_text("ip", &["-4", "route", "show", "default"]).map(|t| parse_default_route_devices(&t)).unwrap_or_default();
    if gw.is_empty()
        && let Ok(t) = std::fs::read_to_string("/proc/net/route")
    {
        gw = parse_proc_route(&t);
    }
    addrs
        .into_iter()
        .map(|(name, ip, mask)| {
            let state = std::fs::read_to_string(format!("/sys/class/net/{name}/operstate")).unwrap_or_default();
            let up = !matches!(state.trim(), "down" | "lowerlayerdown" | "notpresent" | "dormant");
            NetInterface { has_gateway: gw.contains(&name), up, name, ip, mask }
        })
        .collect()
}

/// C# `GetLocalIP`: IPv4 of the first interface that is up and has a gateway.
///
/// `ifs`: interfaces. Returns the address or None.
pub fn local_ip_from(ifs: &[NetInterface]) -> Option<Ipv4Addr> {
    ifs.iter().find(|i| i.up && i.has_gateway).map(|i| i.ip)
}

/// C# `GetLocalIP()` on the live system (with a UDP-route fallback when no tool works).
///
/// Returns the address or None.
pub fn get_local_ip() -> Option<Ipv4Addr> {
    let ifs = list_interfaces();
    if let Some(ip) = local_ip_from(&ifs) {
        return Some(ip);
    }
    if !ifs.is_empty() {
        return None;
    }
    // no tools: ask the routing table (nothing is sent for a UDP connect)
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:53").ok()?;
    match s.local_addr().ok()? {
        SocketAddr::V4(a) => Some(*a.ip()),
        _ => None,
    }
}

/// C# `GetSubnetMask(address)`.
///
/// `address`: local address. `ifs`: interfaces. Returns the mask of the interface owning the address.
pub fn subnet_mask_from(address: Ipv4Addr, ifs: &[NetInterface]) -> Option<Ipv4Addr> {
    ifs.iter().find(|i| i.ip == address).map(|i| i.mask)
}

/// C# `GetSubnetMask` on the live system (/24 when the tools are missing).
///
/// `address`: local address. Returns the mask or None.
pub fn get_subnet_mask(address: Ipv4Addr) -> Option<Ipv4Addr> {
    let ifs = list_interfaces();
    subnet_mask_from(address, &ifs).or(if ifs.is_empty() { Some(Ipv4Addr::new(255, 255, 255, 0)) } else { None })
}

// ---------------------------------------------------------------- ping / mac / name / telnet

/// Extract the round trip ms from `ping` output (Linux `time=0.45 ms`, Windows `time<1ms` / `time=12ms`).
///
/// `text`: ping output. Returns whole milliseconds (truncated) or None without a reply.
pub fn parse_ping_time(text: &str) -> Option<i64> {
    let i = text.find("time=").map(|i| i + 5).or_else(|| text.find("time<").map(|i| i + 5))?;
    let num: String = text[i..].chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    num.parse::<f64>().ok().map(|v| v as i64)
}

/// C# `Ping(ip)`: round trip in ms, or -1 without a reply. TTL 16, 4 byte payload, 5 s timeout.
///
/// `ip`: target. `port`: TCP port used by the fallback probe. Returns ms or -1.
pub fn ping(ip: Ipv4Addr, port: u16) -> i64 {
    let ips = ip.to_string();
    let out = if cfg!(windows) {
        Command::new("ping").args(["-n", "1", "-w", "5000", "-i", "16", "-f", "-l", "4", &ips]).output()
    } else {
        Command::new("ping").args(["-n", "-c", "1", "-W", "5", "-t", "16", "-s", "4", &ips]).output()
    };
    match out {
        Ok(o) => {
            let t = String::from_utf8_lossy(&o.stdout);
            // Windows prints "Destination host unreachable" with exit 0: require a real time field
            parse_ping_time(&t).filter(|_| o.status.success() || t.contains("TTL=")).unwrap_or(-1)
        }
        // no ping tool: TCP connect (refused also proves the host is alive)
        Err(_) => {
            let t0 = Instant::now();
            match TcpStream::connect_timeout(&SocketAddr::from((ip, port)), TIMEOUT) {
                Ok(_) => t0.elapsed().as_millis() as i64,
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => t0.elapsed().as_millis() as i64,
                Err(_) => -1,
            }
        }
    }
}

/// Parse `/proc/net/arp` for one address.
///
/// `text`: file content. `ip`: wanted address. Returns the MAC `aa:bb:..` (lowercase) or None.
pub fn parse_proc_arp(text: &str, ip: Ipv4Addr) -> Option<String> {
    let s = ip.to_string();
    text.lines().skip(1).find_map(|l| {
        let f: Vec<&str> = l.split_whitespace().collect();
        (f.len() >= 4 && f[0] == s && f[3] != "00:00:00:00:00:00").then(|| f[3].to_lowercase())
    })
}

/// Parse Windows `arp -a <ip>` output.
///
/// `text`: output. `ip`: wanted address. Returns the MAC in colon form or None.
pub fn parse_arp_a(text: &str, ip: Ipv4Addr) -> Option<String> {
    let s = ip.to_string();
    text.lines().find_map(|l| {
        let f: Vec<&str> = l.split_whitespace().collect();
        (f.len() >= 2 && f[0] == s && f[1].matches('-').count() == 5).then(|| f[1].replace('-', ":").to_lowercase())
    })
}

/// C# `GetMAC` (ARP cache lookup instead of `SendARP`).
///
/// `ip`: target (should have been contacted before). Returns the MAC or None.
pub fn get_mac(ip: Ipv4Addr) -> Option<String> {
    if cfg!(windows) {
        return run_text("arp", &["-a", &ip.to_string()]).and_then(|t| parse_arp_a(&t, ip));
    }
    std::fs::read_to_string("/proc/net/arp").ok().and_then(|t| parse_proc_arp(&t, ip))
}

/// C# `GetHostName` (reverse lookup). Linux/mac: `getent hosts <ip>`; elsewhere None.
///
/// `ip`: target. Returns the name or None.
pub fn get_host_name(ip: Ipv4Addr) -> Option<String> {
    if cfg!(windows) {
        return None;
    }
    let t = run_text("getent", &["hosts", &ip.to_string()])?;
    t.split_whitespace().nth(1).map(str::to_string)
}

/// C# `Telnet(ip, port)`: TCP connect test.
///
/// `ip`: target. `port`: tcp port. Returns `"Available"` or `"Failed!"`.
pub fn telnet(ip: Ipv4Addr, port: u16) -> String {
    match TcpStream::connect_timeout(&SocketAddr::from((ip, port)), TIMEOUT) {
        Ok(_) => AVAIL.to_string(),
        Err(_) => "Failed!".to_string(),
    }
}

// ---------------------------------------------------------------- ScanResult

#[derive(Debug, Default, Clone)]
struct ResultData {
    ping_time: i64,
    mac: Option<String>,
    host_name: Option<String>,
    telnet: Option<String>,
    notified: bool,
}

struct ResultInner {
    ip: Ipv4Addr,
    port: u16,
    data: Mutex<ResultData>,
    update: Mutex<Vec<Box<dyn Fn() + Send + Sync>>>,
}

/// C# `IPAddressHelper.ScanResult`. Cheap to clone (shared state), safe to read from the UI.
#[derive(Clone)]
pub struct ScanResult {
    inner: Arc<ResultInner>,
}

impl std::fmt::Debug for ScanResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ScanResult({}:{} {})", self.inner.ip, self.inner.port, self)
    }
}

impl ScanResult {
    /// C# ctor with a known ping.
    ///
    /// `address`,`port`: target. `roundtrip_time`: ms of the ping that found it.
    pub fn with_ping(address: Ipv4Addr, port: u16, roundtrip_time: i64) -> Self {
        Self::make(address, port, roundtrip_time, true)
    }

    /// C# ctor for the full scan (nothing known yet).
    ///
    /// `address`,`port`: target.
    pub fn new(address: Ipv4Addr, port: u16) -> Self {
        Self::make(address, port, -1, false)
    }

    fn make(ip: Ipv4Addr, port: u16, ping_time: i64, notified: bool) -> Self {
        let data = ResultData { ping_time, notified, ..Default::default() };
        Self { inner: Arc::new(ResultInner { ip, port, data: Mutex::new(data), update: Mutex::new(Vec::new()) }) }
    }

    /// `IP`.
    pub fn ip(&self) -> Ipv4Addr {
        self.inner.ip
    }

    /// `Port`.
    pub fn port(&self) -> u16 {
        self.inner.port
    }

    fn data(&self) -> std::sync::MutexGuard<'_, ResultData> {
        self.inner.data.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// `MAC`.
    pub fn mac(&self) -> Option<String> {
        self.data().mac.clone()
    }

    /// `HostName`.
    pub fn host_name(&self) -> Option<String> {
        self.data().host_name.clone()
    }

    /// `Telnet` (`"Available"` / `"Failed!"` / None before the test).
    pub fn telnet(&self) -> Option<String> {
        self.data().telnet.clone()
    }

    /// `Ping`: `"{ms}ms"` or None.
    pub fn ping(&self) -> Option<String> {
        let t = self.data().ping_time;
        (t >= 0).then(|| format!("{t}ms"))
    }

    /// C# `HasData`.
    pub fn has_data(&self) -> bool {
        let d = self.data();
        Self::has_data_of(&d)
    }

    fn has_data_of(d: &ResultData) -> bool {
        d.ping_time >= 0 || d.mac.is_some() || d.host_name.is_some() || d.telnet.as_deref() == Some(AVAIL)
    }

    /// `Update` event: called whenever the displayed text changes during a deep scan.
    ///
    /// `f`: handler (runs on a scan thread).
    pub fn subscribe(&self, f: impl Fn() + Send + Sync + 'static) {
        self.inner.update.lock().unwrap_or_else(|e| e.into_inner()).push(Box::new(f));
    }

    /// C# `SetPing`: keep the minimum of known pings.
    fn set_ping(&self, v: i64) {
        if v >= 0 {
            let mut d = self.data();
            // NOTE(upstream): Math.Min(-1, v) keeps -1 when no ping was known before
            d.ping_time = d.ping_time.min(v);
        }
    }

    /// C# `DeepScan(callback, ct)`: ping, MAC, host name and telnet test in parallel.
    ///
    /// `callback`: called once when the result first has data. `cancel`: abort flag.
    pub fn deep_scan(&self, callback: &(dyn Fn(&ScanResult) + Sync), cancel: &AtomicBool) {
        #[derive(Debug)]
        enum Done {
            Ping(i64),
            Mac(Option<String>),
            Host(Option<String>),
            Telnet(String),
        }
        let (ip, port) = (self.inner.ip, self.inner.port);
        let mut old = self.to_string();
        let (tx, rx) = mpsc::channel::<Done>();
        std::thread::scope(|s| {
            let t = tx.clone();
            s.spawn(move || {
                let _ = t.send(Done::Ping(ping(ip, port)));
            });
            let t = tx.clone();
            s.spawn(move || {
                let _ = t.send(Done::Mac(get_mac(ip)));
            });
            let t = tx.clone();
            s.spawn(move || {
                let _ = t.send(Done::Host(get_host_name(ip)));
            });
            let t = tx.clone();
            s.spawn(move || {
                let _ = t.send(Done::Telnet(telnet(ip, port)));
            });
            drop(tx);
            for done in rx {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                match done {
                    Done::Ping(v) => self.set_ping(v),
                    Done::Mac(m) => self.data().mac = m,
                    Done::Host(h) => self.data().host_name = h,
                    Done::Telnet(t) => self.data().telnet = Some(t),
                }
                // first data: tell the scanner
                let first = {
                    let mut d = self.data();
                    let f = !d.notified && Self::has_data_of(&d);
                    if f {
                        d.notified = true;
                    }
                    f
                };
                if first {
                    callback(self);
                }
                let new = self.to_string();
                if self.has_data() && old != new {
                    for h in self.inner.update.lock().unwrap_or_else(|e| e.into_inner()).iter() {
                        h();
                    }
                }
                old = new;
            }
        });
    }
}

impl std::fmt::Display for ScanResult {
    /// C# `ToString`: `Ping + HostName + MAC + Telnet` concatenated.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}{}", self.ping().unwrap_or_default(), self.host_name().unwrap_or_default(), self.mac().unwrap_or_default(), self.telnet().unwrap_or_default())
    }
}

// ---------------------------------------------------------------- IPSegment / scanner

/// C# `IPSegment`: all hosts of a subnet.
#[derive(Debug, Clone)]
pub struct IpSegment {
    network: u32,
    broadcast: u32,
    hosts: Vec<Ipv4Addr>,
}

impl IpSegment {
    /// C# ctor.
    ///
    /// `ip`: local address. `mask`: subnet mask. Builds the host list `network+1 .. broadcast-1`.
    pub fn new(ip: Ipv4Addr, mask: Ipv4Addr) -> Self {
        let (ipw, mw) = (to_uint32(ip), to_uint32(mask));
        let network = ipw & mw;
        let broadcast = network.wrapping_add(!mw);
        let (net, broad) = (reverse_bytes_u32(network), reverse_bytes_u32(broadcast));
        let mut hosts = Vec::new();
        let mut host = net.wrapping_add(1);
        while host < broad {
            hosts.push(to_ip_address(reverse_bytes_u32(host)));
            host += 1;
        }
        Self { network, broadcast, hosts }
    }

    /// `NetworkAddress`.
    pub fn network_address(&self) -> Ipv4Addr {
        to_ip_address(self.network)
    }

    /// `BroadcastAddress`.
    pub fn broadcast_address(&self) -> Ipv4Addr {
        to_ip_address(self.broadcast)
    }

    /// `HostToScan`.
    pub fn host_to_scan(&self) -> &[Ipv4Addr] {
        &self.hosts
    }
}

/// Result callback type.
pub type ResultCallback<'a> = &'a (dyn Fn(&ScanResult) + Sync);
/// Progress callback `(phase, count, total)`; phase 0 = fast scan, 1 = deep scan.
pub type ProgressCallback<'a> = &'a (dyn Fn(i32, i32, i32) + Sync);

/// C# `IPSegmentScanner`.
pub struct IpSegmentScanner<'a> {
    segment: IpSegment,
    callback: ResultCallback<'a>,
    progress: ProgressCallback<'a>,
    cancel: &'a AtomicBool,
    port: u16,
    list: Mutex<Vec<ScanResult>>,
}

impl<'a> IpSegmentScanner<'a> {
    /// C# ctor.
    ///
    /// `segment`: hosts to scan. `callback`: new result found. `progress`: progress reports.
    /// `port`: telnet port. `cancel`: abort flag (C# `CancellationToken`).
    pub fn new(segment: IpSegment, callback: ResultCallback<'a>, progress: ProgressCallback<'a>, port: u16, cancel: &'a AtomicBool) -> Self {
        Self { segment, callback, progress, cancel, port, list: Mutex::new(Vec::new()) }
    }

    /// C# `FullScan`: every host gets a deep scan.
    pub fn full_scan(&self) {
        {
            let mut l = self.list.lock().unwrap_or_else(|e| e.into_inner());
            for h in self.segment.host_to_scan() {
                l.push(ScanResult::new(*h, self.port));
            }
        }
        self.analize();
    }

    /// C# `FastScan`: ping everything, deep scan the responders.
    pub fn fast_scan(&self) {
        self.build_responding_list();
        self.analize();
    }

    /// C# `BuildRespondingList` with a worker pool.
    fn build_responding_list(&self) {
        let hosts = self.segment.host_to_scan();
        let total = hosts.len() as i32;
        let next = AtomicUsize::new(0);
        let resp = AtomicUsize::new(0);
        (self.progress)(0, 0, total);
        let workers = hosts.len().clamp(1, 48);
        std::thread::scope(|s| {
            for _ in 0..workers {
                s.spawn(|| {
                    loop {
                        if self.cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        let i = next.fetch_add(1, Ordering::SeqCst);
                        let Some(h) = hosts.get(i) else { break };
                        let rtt = ping(*h, self.port);
                        if rtt >= 0 {
                            let sr = ScanResult::with_ping(*h, self.port, rtt);
                            self.list.lock().unwrap_or_else(|e| e.into_inner()).push(sr.clone());
                            (self.callback)(&sr);
                        }
                        let n = resp.fetch_add(1, Ordering::SeqCst) + 1;
                        (self.progress)(0, n as i32, total);
                    }
                });
            }
        });
        (self.progress)(0, resp.load(Ordering::SeqCst) as i32, total);
    }

    /// C# `Analize`: parallel deep scan of the collected results.
    fn analize(&self) {
        let list: Vec<ScanResult> = self.list.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let total = list.len() as i32;
        let done = AtomicUsize::new(0);
        (self.progress)(1, 0, total);
        if self.cancel.load(Ordering::Relaxed) {
            return;
        }
        let next = AtomicUsize::new(0);
        let workers = list.len().clamp(1, 32);
        std::thread::scope(|s| {
            for _ in 0..workers {
                s.spawn(|| {
                    loop {
                        if self.cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        let i = next.fetch_add(1, Ordering::SeqCst);
                        let Some(sr) = list.get(i) else { break };
                        sr.deep_scan(self.callback, self.cancel);
                        let n = done.fetch_add(1, Ordering::SeqCst) + 1;
                        (self.progress)(1, n as i32, total);
                    }
                });
            }
        });
    }
}

/// C# `IPAddressHelper.ScanIP`: scan the local subnet (blocking).
///
/// `callback`: called for each new result. `progress`: `(phase, count, total)`. `cancel`: abort
/// flag. `port`: telnet port. `param_ip`: local address to use instead of the detected one.
/// Returns Err with the C# exception texts when no adapter / mask is found.
pub fn scan_ip(callback: ResultCallback<'_>, progress: ProgressCallback<'_>, cancel: &AtomicBool, port: u16, param_ip: Option<Ipv4Addr>) -> Result<(), String> {
    let local = match param_ip.or_else(get_local_ip) {
        Some(ip) => ip,
        None => return Err(format!("Can't find network adapter for IP {}", param_ip.map(|i| i.to_string()).unwrap_or_default())),
    };
    let mask = get_subnet_mask(local).ok_or_else(|| format!("Can't find subnet mask for IP {local}"))?;
    let segment = IpSegment::new(local, mask);
    IpSegmentScanner::new(segment, callback, progress, port, cancel).fast_scan();
    Ok(())
}

// ---------------------------------------------------------------- DiscoveryForm logic

/// `ComWrapper.WrapperType` values the discovery form distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryProtocol {
    UsbSerial,
    Telnet,
    LaserWebEsp8266,
}

impl DiscoveryProtocol {
    /// Parse the stored setting name (`"ComWrapper Protocol"`).
    ///
    /// `name`: enum name text. Unknown names count as serial like C# `else` branches.
    pub fn from_setting_name(name: &str) -> Self {
        match name {
            "Telnet" => Self::Telnet,
            "LaserWebESP8266" => Self::LaserWebEsp8266,
            _ => Self::UsbSerial,
        }
    }
}

/// `DiscoveryForm` ctor: initial port number.
///
/// `p`: current protocol. `is_longer_board`: `core.IsLongerBoard`. Returns 8847/23 (telnet), 81 (websocket) or 666.
pub fn default_port(p: DiscoveryProtocol, is_longer_board: bool) -> u16 {
    match p {
        DiscoveryProtocol::Telnet => {
            if is_longer_board {
                8847
            } else {
                23
            }
        }
        DiscoveryProtocol::LaserWebEsp8266 => 81,
        DiscoveryProtocol::UsbSerial => 666,
    }
}

/// `DiscoveryForm.ReturnItem` value.
///
/// `p`: current protocol. `r`: chosen result. Returns `ip:port` (telnet), `ws://ip:port/`
/// (websocket) or None (serial: nothing is returned).
pub fn result_address(p: DiscoveryProtocol, r: &ScanResult) -> Option<String> {
    match p {
        DiscoveryProtocol::Telnet => Some(format!("{}:{}", r.ip(), r.port())),
        DiscoveryProtocol::LaserWebEsp8266 => Some(format!("ws://{}:{}/", r.ip(), r.port())),
        DiscoveryProtocol::UsbSerial => None,
    }
}

/// `DiscoveryForm.ScanProgress` label text.
///
/// `phase`: 0 fast, 1 deep. `count`,`total`: progress. Returns the label, None for other phases.
pub fn progress_label(phase: i32, count: i32, total: i32) -> Option<String> {
    match phase {
        0 => Some(format!("Fast scan: {count}/{total}")),
        1 => Some(format!("Deep scan: {count}/{total}")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> Ipv4Addr {
        s.parse().unwrap()
    }

    #[test]
    fn byte_helpers() {
        assert_eq!(reverse_bytes_u16(0x1234), 0x3412);
        assert_eq!(reverse_bytes_u32(0x0102_0304), 0x0403_0201);
        assert_eq!(reverse_bytes_u64(0x0102_0304_0506_0708), 0x0807_0605_0403_0201);
        assert_eq!(to_uint32(ip("1.2.3.4")), 0x0403_0201);
        assert_eq!(to_ip_address(0x0403_0201), ip("1.2.3.4"));
    }

    #[test]
    fn address_math() {
        let (a, m) = (ip("192.168.1.77"), ip("255.255.255.0"));
        assert_eq!(get_network_address(a, m), ip("192.168.1.0"));
        assert_eq!(get_broadcast_address(a, m), ip("192.168.1.255"));
        assert!(is_in_same_subnet(ip("192.168.1.5"), a, m));
        assert!(!is_in_same_subnet(ip("192.168.2.5"), a, m));
    }

    #[test]
    fn segment_hosts() {
        let s = IpSegment::new(ip("10.0.0.77"), ip("255.255.255.0"));
        assert_eq!(s.host_to_scan().len(), 254);
        assert_eq!(s.host_to_scan()[0], ip("10.0.0.1"));
        assert_eq!(s.host_to_scan()[253], ip("10.0.0.254"));
        assert_eq!(s.network_address(), ip("10.0.0.0"));
        assert_eq!(s.broadcast_address(), ip("10.0.0.255"));
        let s = IpSegment::new(ip("10.0.0.77"), ip("255.255.255.252"));
        assert_eq!(s.host_to_scan().len(), 2);
        let s = IpSegment::new(ip("172.16.5.1"), ip("255.255.0.0"));
        assert_eq!(s.host_to_scan().len(), 65534);
    }

    #[test]
    fn parse_linux_tools() {
        let a = "1: lo    inet 127.0.0.1/8 scope host lo\\       valid_lft forever\n2: wlan0    inet 192.168.178.23/24 brd 192.168.178.255 scope global dynamic wlan0\\       valid_lft 600sec\n3: br-1@if4    inet 172.17.0.1/16 brd 172.17.255.255 scope global br-1\n";
        let v = parse_ip_addr(a);
        assert_eq!(v.len(), 3);
        assert_eq!(v[1], ("wlan0".to_string(), ip("192.168.178.23"), ip("255.255.255.0")));
        assert_eq!(v[2].0, "br-1");
        assert_eq!(v[2].2, ip("255.255.0.0"));
        assert_eq!(parse_default_route_devices("default via 192.168.178.1 dev wlan0 proto dhcp metric 600\n"), vec!["wlan0"]);
        let r = "Iface\tDestination\tGateway\tFlags\nwlan0\t00000000\t0102A8C0\t0003\neth1\t0000A8C0\t00000000\t0001\n";
        assert_eq!(parse_proc_route(r), vec!["wlan0"]);
        let ifs: Vec<NetInterface> = v.into_iter().map(|(n, i, m)| NetInterface { has_gateway: n == "wlan0", name: n, ip: i, mask: m, up: true }).collect();
        assert_eq!(local_ip_from(&ifs), Some(ip("192.168.178.23")));
        assert_eq!(subnet_mask_from(ip("192.168.178.23"), &ifs), Some(ip("255.255.255.0")));
        assert_eq!(subnet_mask_from(ip("9.9.9.9"), &ifs), None);
    }

    #[test]
    fn parse_windows_ipconfig() {
        let t = "Windows IP Configuration\n\nEthernet adapter Ethernet:\n\n   Connection-specific DNS Suffix  . : lan\n   IPv4 Address. . . . . . . . . . . : 192.168.1.20\n   Subnet Mask . . . . . . . . . . . : 255.255.255.0\n   Default Gateway . . . . . . . . . : 192.168.1.1\n\nEthernet adapter VPN:\n\n   IPv4 Address. . . . . . . . . . . : 10.1.1.2(Preferred)\n   Subnet Mask . . . . . . . . . . . : 255.255.255.255\n";
        let v = parse_ipconfig(t);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].ip, ip("192.168.1.20"));
        assert!(v[0].has_gateway && !v[1].has_gateway);
        assert_eq!(v[1].ip, ip("10.1.1.2"));
    }

    #[test]
    fn parse_ping_arp() {
        assert_eq!(parse_ping_time("64 bytes from 1.2.3.4: icmp_seq=1 ttl=64 time=0.45 ms"), Some(0));
        assert_eq!(parse_ping_time("Reply from 1.2.3.4: bytes=4 time=12ms TTL=64"), Some(12));
        assert_eq!(parse_ping_time("Reply from 1.2.3.4: bytes=4 time<1ms TTL=64"), Some(1));
        assert_eq!(parse_ping_time("100% packet loss"), None);
        let arp = "IP address       HW type     Flags       HW address            Mask     Device\n192.168.1.1      0x1         0x2         AA:BB:CC:DD:EE:FF     *        wlan0\n192.168.1.9      0x1         0x0         00:00:00:00:00:00     *        wlan0\n";
        assert_eq!(parse_proc_arp(arp, ip("192.168.1.1")).as_deref(), Some("aa:bb:cc:dd:ee:ff"));
        assert_eq!(parse_proc_arp(arp, ip("192.168.1.9")), None);
        let a = "Interface: 192.168.1.20 --- 0x4\n  Internet Address      Physical Address      Type\n  192.168.1.1           aa-bb-cc-dd-ee-ff     dynamic\n";
        assert_eq!(parse_arp_a(a, ip("192.168.1.1")).as_deref(), Some("aa:bb:cc:dd:ee:ff"));
    }

    #[test]
    fn scan_result_text_and_flags() {
        let r = ScanResult::with_ping(ip("1.2.3.4"), 23, 7);
        assert!(r.has_data());
        assert_eq!(r.ping().as_deref(), Some("7ms"));
        assert_eq!(r.to_string(), "7ms");
        let n = ScanResult::new(ip("1.2.3.4"), 23);
        assert!(!n.has_data());
        n.set_ping(5);
        // NOTE(upstream): Math.Min with the initial -1 keeps -1
        assert!(n.ping().is_none());
    }

    #[test]
    fn telnet_probe_on_loopback() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        assert_eq!(telnet(ip("127.0.0.1"), port), "Available");
    }

    #[test]
    fn deep_scan_loopback_notifies() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let r = ScanResult::new(ip("127.0.0.1"), port);
        let hits = AtomicUsize::new(0);
        let cancel = AtomicBool::new(false);
        r.deep_scan(&|_| {
            hits.fetch_add(1, Ordering::SeqCst);
        }, &cancel);
        // telnet is available on loopback, so the first-data callback fired exactly once
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert_eq!(r.telnet().as_deref(), Some("Available"));
    }

    #[test]
    fn discovery_helpers() {
        assert_eq!(default_port(DiscoveryProtocol::Telnet, true), 8847);
        assert_eq!(default_port(DiscoveryProtocol::Telnet, false), 23);
        assert_eq!(default_port(DiscoveryProtocol::LaserWebEsp8266, false), 81);
        assert_eq!(default_port(DiscoveryProtocol::UsbSerial, false), 666);
        let r = ScanResult::with_ping(ip("1.2.3.4"), 81, 1);
        assert_eq!(result_address(DiscoveryProtocol::LaserWebEsp8266, &r).as_deref(), Some("ws://1.2.3.4:81/"));
        assert_eq!(result_address(DiscoveryProtocol::Telnet, &r).as_deref(), Some("1.2.3.4:81"));
        assert_eq!(result_address(DiscoveryProtocol::UsbSerial, &r), None);
        assert_eq!(progress_label(1, 3, 9).as_deref(), Some("Deep scan: 3/9"));
        assert_eq!(DiscoveryProtocol::from_setting_name("Telnet"), DiscoveryProtocol::Telnet);
    }

    #[test]
    fn scan_unknown_adapter_or_cancelled() {
        let cancel = AtomicBool::new(true);
        // address that no interface owns: error, or (no tools) a cancelled /24 scan
        let res = scan_ip(&|_| {}, &|_, _, _| {}, &cancel, 23, Some(ip("203.0.113.9")));
        if let Err(e) = res {
            assert!(e.starts_with("Can't find subnet mask for IP"));
        }
    }
}
