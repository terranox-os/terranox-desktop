// SPDX-License-Identifier: Apache-2.0
//! Mock Sentinel daemon — produces realistic security data.
//!
//! Listens on a Unix domain socket and streams SentinelState
//! updates to connected dashboards. Simulates capability operations,
//! package verification, and security alerts.
//!
//! Usage: sentinel-mock
//!   Starts on /tmp/sentinel.sock, updates every 2 seconds.

mod protocol;

use protocol::*;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixListener;
use tokio::sync::watch;
use tokio::time::{self, Duration};

/// Generate a mock SentinelState with evolving data.
fn generate_state(tick: u64) -> SentinelState {
    let base_caps = 42 + (tick % 10) as u32;
    let denials = if tick % 15 == 0 { 1 } else { 0 };

    let level = if denials > 0 && tick % 30 < 5 {
        SecurityLevel::Warning
    } else {
        SecurityLevel::Secure
    };

    let mut alerts = Vec::new();

    if tick % 20 == 0 {
        alerts.push(Alert {
            severity: AlertSeverity::Info,
            message: "Capability audit completed — 0 anomalies".into(),
            source: "sentinel".into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    if denials > 0 {
        alerts.push(Alert {
            severity: AlertSeverity::Warning,
            message: format!("Permission denied: pid {} attempted CAP_WRITE on protected region", 100 + tick),
            source: "cap_check".into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    if tick % 50 == 0 && tick > 0 {
        alerts.push(Alert {
            severity: AlertSeverity::Critical,
            message: "Unsigned binary blocked: /tmp/suspicious".into(),
            source: "sigil_verify".into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    SentinelState {
        level,
        mode: StrataMode::Hardened,
        caps: CapabilityStats {
            active: base_caps,
            granted: 128 + tick as u32,
            revoked: 12 + (tick / 5) as u32,
            denials: denials + (tick / 15) as u32,
        },
        packages: PackageStats {
            total: 247,
            verified: 245,
            sbom_coverage: 247,
            vulnerabilities: if tick % 40 < 10 { 2 } else { 0 },
        },
        alerts,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Remove stale socket
    let _ = std::fs::remove_file(SENTINEL_SOCKET);

    let listener = UnixListener::bind(SENTINEL_SOCKET)?;
    eprintln!("Sentinel mock daemon listening on {}", SENTINEL_SOCKET);

    // State broadcaster
    let (tx, _rx) = watch::channel(generate_state(0));
    let tx = Arc::new(tx);

    // State update task — generates new state every 2 seconds
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let mut tick: u64 = 0;
        let mut interval = time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            tick += 1;
            let state = generate_state(tick);
            let _ = tx_clone.send(state);
        }
    });

    // Accept connections
    loop {
        let (stream, _addr) = listener.accept().await?;
        let mut rx = tx.subscribe();
        eprintln!("Dashboard connected");

        tokio::spawn(async move {
            let (_, mut writer) = tokio::io::split(stream);

            // Send initial state
            let initial = rx.borrow().clone();
            let resp = DaemonResponse::State { state: initial };
            let mut json = serde_json::to_string(&resp).unwrap();
            json.push('\n');
            if writer.write_all(json.as_bytes()).await.is_err() {
                return;
            }

            // Stream updates
            loop {
                if rx.changed().await.is_err() {
                    break;
                }
                let state = rx.borrow().clone();
                let resp = DaemonResponse::State { state };
                let mut json = serde_json::to_string(&resp).unwrap();
                json.push('\n');
                if writer.write_all(json.as_bytes()).await.is_err() {
                    eprintln!("Dashboard disconnected");
                    break;
                }
            }
        });
    }
}
