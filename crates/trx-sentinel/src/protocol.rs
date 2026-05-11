// SPDX-License-Identifier: Apache-2.0
//! Sentinel IPC protocol — shared types between daemon and dashboard.
//!
//! Communication is via Unix domain socket using newline-delimited JSON.
//! The dashboard connects and receives a stream of `SentinelState` updates.

use serde::{Deserialize, Serialize};

/// Socket path for Sentinel IPC.
pub const SENTINEL_SOCKET: &str = "/tmp/sentinel.sock";

/// Overall system security status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityLevel {
    /// All checks passing, no alerts.
    Secure,
    /// Alerts pending review.
    Warning,
    /// Active security incident or daemon failure.
    Critical,
}

/// Active strata mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrataMode {
    Desktop,
    Hardened,
    Sentinel,
    Developer,
}

/// Full Sentinel state snapshot — sent on connect and on change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelState {
    /// Current security level.
    pub level: SecurityLevel,

    /// Active strata mode.
    pub mode: StrataMode,

    /// Capability system stats.
    pub caps: CapabilityStats,

    /// Package verification stats.
    pub packages: PackageStats,

    /// Recent security alerts.
    pub alerts: Vec<Alert>,

    /// Timestamp of this snapshot (RFC 3339).
    pub timestamp: String,
}

/// Capability system statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityStats {
    /// Total active capabilities.
    pub active: u32,
    /// Capabilities granted this session.
    pub granted: u32,
    /// Capabilities revoked this session.
    pub revoked: u32,
    /// Permission denials this session.
    pub denials: u32,
}

/// Package verification statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageStats {
    /// Total installed packages.
    pub total: u32,
    /// Packages with valid signatures.
    pub verified: u32,
    /// Packages with SBOMs.
    pub sbom_coverage: u32,
    /// Known vulnerabilities (from VEX).
    pub vulnerabilities: u32,
}

/// Security alert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert severity.
    pub severity: AlertSeverity,
    /// Short description.
    pub message: String,
    /// Source (e.g., "cap_check", "sigil_verify", "sentinel").
    pub source: String,
    /// Timestamp (RFC 3339).
    pub timestamp: String,
}

/// Alert severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Request from dashboard to daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DashboardRequest {
    /// Request current state snapshot.
    GetState,
    /// Request alert acknowledgment.
    AckAlert { index: usize },
    /// Request strata mode change.
    SwitchMode { mode: StrataMode },
}

/// Response from daemon to dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    /// Full state snapshot.
    State { state: SentinelState },
    /// Error response.
    Error { message: String },
}
