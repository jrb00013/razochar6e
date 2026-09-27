//! TP-Link Kasa plug control: legacy XOR (TCP 9999) + python-kasa helper (KLAP).

use crate::error::{RazError, RazResult};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const KASA_PORT: u16 = 9999;
const XOR_KEY: u8 = 171;

#[derive(Debug, Clone)]
pub struct KasaAuth {
    pub username: Option<String>,
    pub password: Option<String>,
}

impl KasaAuth {
    pub fn from_env_and_opts(username: Option<String>, password: Option<String>) -> Self {
        Self {
            username: username.or_else(|| std::env::var("KASA_USERNAME").ok()),
            password: password.or_else(|| std::env::var("KASA_PASSWORD").ok()),
        }
    }

    pub fn has_creds(&self) -> bool {
        self.username.as_ref().is_some_and(|u| !u.is_empty())
            && self.password.as_ref().is_some_and(|p| !p.is_empty())
    }

    pub fn missing_creds_hint(&self) -> Option<&'static str> {
        if self.has_creds() {
            None
        } else {
            Some(
                "this plug needs TP-Link account credentials — set KASA_USERNAME and KASA_PASSWORD \
                 (same email/password as the Kasa app)",
            )
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlugInfo {
    pub host: String,
    pub alias: Option<String>,
    pub model: Option<String>,
    pub is_on: Option<bool>,
}

pub fn encrypt(request: &[u8]) -> Vec<u8> {
    let mut key = XOR_KEY;
    let mut out = Vec::with_capacity(4 + request.len());
    out.extend_from_slice(&(request.len() as u32).to_be_bytes());
    for &b in request {
        key ^= b;
        out.push(key);
    }
    out
}

pub fn decrypt(ciphertext: &[u8]) -> Vec<u8> {
    let mut key = XOR_KEY;
    let mut out = Vec::with_capacity(ciphertext.len());
    for &b in ciphertext {
        out.push(b ^ key);
        key = b;
    }
    out
}

fn xor_query(host: &str, json: &str) -> RazResult<Value> {
    let addr: SocketAddr =
        format!("{host}:{KASA_PORT}")
            .parse()
            .map_err(|e| RazError::Backend {
                backend: "kasa".into(),
                message: format!("bad host {host}: {e}"),
            })?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| {
        RazError::Backend {
            backend: "kasa".into(),
            message: format!("connect {addr}: {e}"),
        }
    })?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
    stream
        .write_all(&encrypt(json.as_bytes()))
        .map_err(|e| RazError::Backend {
            backend: "kasa".into(),
            message: format!("send: {e}"),
        })?;

    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .map_err(|e| RazError::Backend {
            backend: "kasa".into(),
            message: format!("read len: {e}"),
        })?;
    let n = u32::from_be_bytes(len_buf) as usize;
    if n == 0 || n > 1_000_000 {
        return Err(RazError::Backend {
            backend: "kasa".into(),
            message: format!("implausible response length {n}"),
        });
    }
    let mut body = vec![0u8; n];
    stream
        .read_exact(&mut body)
        .map_err(|e| RazError::Backend {
            backend: "kasa".into(),
            message: format!("read body: {e}"),
        })?;
    let plain = decrypt(&body);
    serde_json::from_slice(&plain).map_err(|e| RazError::Backend {
        backend: "kasa".into(),
        message: format!("json: {e}"),
    })
}

/// Legacy TCP 9999 discovery (broadcast). Returns hosts that answered.
pub fn discover_legacy(timeout: Duration) -> RazResult<Vec<PlugInfo>> {
    let sock = UdpSocket::bind("0.0.0.0:0").map_err(RazError::Io)?;
    sock.set_broadcast(true).map_err(RazError::Io)?;
    sock.set_read_timeout(Some(timeout)).map_err(RazError::Io)?;
    let payload = encrypt(br#"{"system":{"get_sysinfo":{}}}"#);
    let _ = sock.send_to(&payload, "255.255.255.255:9999");

    let mut found = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match sock.recv_from(&mut buf) {
            Ok((n, addr)) => {
                if n < 4 {
                    continue;
                }
                // UDP responses are length-prefixed encrypted payloads on some firmwares,
                // and raw encrypted payloads on others.
                let cipher = if n >= 4 {
                    let claimed = u32::from_be_bytes(buf[0..4].try_into().unwrap()) as usize;
                    if claimed + 4 == n {
                        &buf[4..n]
                    } else {
                        &buf[..n]
                    }
                } else {
                    &buf[..n]
                };
                let plain = decrypt(cipher);
                if let Ok(v) = serde_json::from_slice::<Value>(&plain) {
                    let info = v
                        .pointer("/system/get_sysinfo")
                        .cloned()
                        .unwrap_or(Value::Null);
                    found.push(PlugInfo {
                        host: addr.ip().to_string(),
                        alias: info
                            .get("alias")
                            .and_then(|x| x.as_str())
                            .map(str::to_string),
                        model: info
                            .get("model")
                            .and_then(|x| x.as_str())
                            .map(str::to_string),
                        is_on: info
                            .get("relay_state")
                            .and_then(|x| x.as_u64())
                            .map(|u| u == 1),
                    });
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(RazError::Io(e)),
        }
    }
    Ok(found)
}

fn find_helper_script() -> RazResult<PathBuf> {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest).join("scripts/kasa_plug.py");
        if p.exists() {
            return Ok(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in [
                dir.join("kasa_plug.py"),
                dir.join("../scripts/kasa_plug.py"),
                dir.join("../../scripts/kasa_plug.py"),
            ] {
                if cand.exists() {
                    return Ok(cand);
                }
            }
        }
    }
    for cand in [
        PathBuf::from("scripts/kasa_plug.py"),
        PathBuf::from("./scripts/kasa_plug.py"),
    ] {
        if cand.exists() {
            return Ok(cand);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".local/share/razochar6e/scripts/kasa_plug.py");
        if p.exists() {
            return Ok(p);
        }
    }
    Err(RazError::Backend {
        backend: "kasa".into(),
        message: "scripts/kasa_plug.py not found (needed for KLAP / modern Kasa firmware)".into(),
    })
}

fn python_bins() -> Vec<&'static str> {
    #[cfg(windows)]
    {
        vec!["python", "py"]
    }
    #[cfg(not(windows))]
    {
        // Prefer Windows Python from WSL when available (same LAN view as the host).
        if std::env::var_os("WSL_DISTRO_NAME").is_some() {
            vec!["python.exe", "python3", "python"]
        } else {
            vec!["python3", "python"]
        }
    }
}

fn run_helper(args: &[&str], auth: &KasaAuth) -> RazResult<Value> {
    let script = find_helper_script()?;
    let mut cmd_args: Vec<String> = vec![script.display().to_string()];
    if let Some(u) = &auth.username {
        cmd_args.push("--username".into());
        cmd_args.push(u.clone());
    }
    if let Some(p) = &auth.password {
        cmd_args.push("--password".into());
        cmd_args.push(p.clone());
    }
    for a in args {
        cmd_args.push((*a).to_string());
    }

    let mut last_err = String::new();
    for bin in python_bins() {
        let out = Command::new(bin).args(&cmd_args).output();
        let out = match out {
            Ok(o) => o,
            Err(e) => {
                last_err = format!("{bin}: {e}");
                continue;
            }
        };
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if out.status.success() {
            return serde_json::from_str(stdout.trim()).map_err(|e| RazError::Backend {
                backend: "kasa".into(),
                message: format!("helper json: {e}; stdout={stdout}"),
            });
        }
        last_err = format!(
            "{bin} exit {}: {}",
            out.status,
            if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            }
        );
        // If python ran but kasa auth failed, don't keep trying other bins.
        if last_err.contains("Authentication") || last_err.contains("credentials") {
            break;
        }
    }
    Err(RazError::Backend {
        backend: "kasa".into(),
        message: format!(
            "python-kasa helper failed ({last_err}). Install with `pip install python-kasa`.{}",
            if auth.has_creds() {
                String::new()
            } else {
                format!(" {}", auth.missing_creds_hint().unwrap_or(""))
            }
        ),
    })
}

pub fn discover(auth: &KasaAuth) -> RazResult<Vec<PlugInfo>> {
    let mut plugs = discover_legacy(Duration::from_secs(3)).unwrap_or_default();
    if let Ok(v) = run_helper(&["discover"], auth) {
        if let Some(arr) = v.get("devices").and_then(|d| d.as_array()) {
            for row in arr {
                let host = row
                    .get("host")
                    .and_then(|h| h.as_str())
                    .unwrap_or("")
                    .to_string();
                if host.is_empty() {
                    continue;
                }
                if plugs.iter().any(|p| p.host == host) {
                    continue;
                }
                plugs.push(PlugInfo {
                    host,
                    alias: row
                        .get("alias")
                        .and_then(|x| x.as_str())
                        .map(str::to_string),
                    model: row
                        .get("model")
                        .and_then(|x| x.as_str())
                        .map(str::to_string),
                    is_on: row.get("is_on").and_then(|x| x.as_bool()),
                });
            }
        }
    }
    Ok(plugs)
}

fn xor_relay_state(host: &str) -> RazResult<bool> {
    let v = xor_query(host, r#"{"system":{"get_sysinfo":{}}}"#)?;
    let state = v
        .pointer("/system/get_sysinfo/relay_state")
        .and_then(|x| x.as_u64())
        .ok_or_else(|| RazError::Backend {
            backend: "kasa".into(),
            message: "no relay_state in get_sysinfo".into(),
        })?;
    Ok(state == 1)
}

fn xor_set_relay(host: &str, on: bool) -> RazResult<()> {
    let state = if on { 1 } else { 0 };
    let json = format!(r#"{{"system":{{"set_relay_state":{{"state":{state}}}}}}}"#);
    let v = xor_query(host, &json)?;
    let err = v
        .pointer("/system/set_relay_state/err_code")
        .and_then(|x| x.as_i64())
        .unwrap_or(-1);
    if err != 0 {
        return Err(RazError::Backend {
            backend: "kasa".into(),
            message: format!("set_relay_state err_code={err}: {v}"),
        });
    }
    Ok(())
}

pub fn is_on(host: &str, auth: &KasaAuth) -> RazResult<bool> {
    match xor_relay_state(host) {
        Ok(on) => Ok(on),
        Err(_) => {
            let v = run_helper(&["state", "--host", host], auth)?;
            v.get("is_on")
                .and_then(|x| x.as_bool())
                .ok_or_else(|| RazError::Backend {
                    backend: "kasa".into(),
                    message: format!("state response missing is_on: {v}"),
                })
        }
    }
}

pub fn set_on(host: &str, on: bool, auth: &KasaAuth) -> RazResult<()> {
    if xor_set_relay(host, on).is_ok() {
        return Ok(());
    }
    let cmd = if on { "on" } else { "off" };
    let v = run_helper(&[cmd, "--host", host], auth)?;
    let got = v.get("is_on").and_then(|x| x.as_bool());
    if got != Some(on) {
        return Err(RazError::Backend {
            backend: "kasa".into(),
            message: format!("failed to set relay on={on}: {v}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_roundtrip() {
        let msg = br#"{"system":{"get_sysinfo":{}}}"#;
        let enc = encrypt(msg);
        assert_eq!(&enc[..4], &(msg.len() as u32).to_be_bytes());
        let dec = decrypt(&enc[4..]);
        assert_eq!(dec, msg);
    }
}
