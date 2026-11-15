//! Pairing, session tokens, policy and the append-only audit chain.
//!
//! A remote session is started by the host, never by an agent. The pairing code
//! is one-shot and short-lived; the session token is a separate high-entropy
//! bearer value bound to that session. Every denial is audited.

use std::fmt;

use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::encode::Codec;
use crate::ladder::{ClientCaps, Tier};

pub const REMOTE_HOST_SCOPE: &str = "remote.host";
pub const REMOTE_VIEW_SCOPE: &str = "remote.view";
pub const REMOTE_INPUT_SCOPE: &str = "remote.input";
pub const REMOTE_CLIPBOARD_SCOPE: &str = "remote.clipboard";

pub const DEFAULT_PAIRING_TTL_MS: u64 = 120_000;
pub const DEFAULT_SESSION_TTL_MS: u64 = 3_600_000;
pub const MAX_BITRATE_KBPS: u32 = 25_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPolicy {
    pub allow_input: bool,
    /// Clipboard is off by default and requires an explicit per-session toggle.
    pub allow_clipboard: bool,
    pub allow_audio: bool,
    pub max_bitrate_kbps: u32,
    pub session_ttl_ms: u64,
}

impl Default for SessionPolicy {
    fn default() -> Self {
        Self {
            allow_input: true,
            allow_clipboard: false,
            allow_audio: false,
            max_bitrate_kbps: 12_000,
            session_ttl_ms: DEFAULT_SESSION_TTL_MS,
        }
    }
}

impl SessionPolicy {
    pub fn clamped(mut self) -> Self {
        self.max_bitrate_kbps = self.max_bitrate_kbps.clamp(500, MAX_BITRATE_KBPS);
        self.session_ttl_ms = self.session_ttl_ms.clamp(10_000, 24 * 60 * 60 * 1_000);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingCode {
    pub code: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub used: bool,
}

impl PairingCode {
    pub fn display(&self) -> String {
        if self.code.len() == 8 {
            format!("{}-{}", &self.code[..4], &self.code[4..])
        } else {
            self.code.clone()
        }
    }

    pub fn expired(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_at_ms
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairingError {
    Invalid,
    Expired,
    Used,
}

impl fmt::Display for PairingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => write!(f, "invalid pairing code"),
            Self::Expired => write!(f, "pairing code expired"),
            Self::Used => write!(f, "pairing code already used"),
        }
    }
}

impl std::error::Error for PairingError {}

/// One-shot pairing registry. Codes are generated from 40 random bits with a
/// Crockford-like alphabet that avoids ambiguous characters.
#[derive(Clone, Debug, Default)]
pub struct PairingRegistry {
    codes: Vec<PairingCode>,
    ttl_ms: u64,
}

const ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

impl PairingRegistry {
    pub fn new(ttl_ms: u64) -> Self {
        Self {
            codes: Vec::new(),
            ttl_ms: ttl_ms.max(1_000),
        }
    }

    pub fn issue(&mut self, now_ms: u64) -> PairingCode {
        let mut bits = [0u8; 8];
        rand::rng().fill_bytes(&mut bits);
        let mut code = String::with_capacity(8);
        let mut accumulator: u32 = 0;
        let mut available = 0u32;
        for byte in bits {
            accumulator = (accumulator << 8) | u32::from(byte);
            available += 8;
            while available >= 5 && code.len() < 8 {
                let index = ((accumulator >> (available - 5)) & 0x1f) as usize;
                code.push(ALPHABET[index] as char);
                available -= 5;
            }
        }
        let code = PairingCode {
            code,
            issued_at_ms: now_ms,
            expires_at_ms: now_ms + self.ttl_ms,
            used: false,
        };
        self.codes.push(code.clone());
        code
    }

    pub fn redeem(&mut self, now_ms: u64, input: &str) -> Result<PairingCode, PairingError> {
        let normalized: String = input
            .chars()
            .filter(|character| !matches!(character, '-' | ' ' | '_'))
            .flat_map(char::to_uppercase)
            .collect();
        if normalized.len() != 8 || !normalized.bytes().all(|byte| ALPHABET.contains(&byte)) {
            return Err(PairingError::Invalid);
        }
        let Some(index) = self
            .codes
            .iter()
            .position(|candidate| candidate.code == normalized)
        else {
            return Err(PairingError::Invalid);
        };
        let candidate = &mut self.codes[index];
        if candidate.used {
            return Err(PairingError::Used);
        }
        if candidate.expired(now_ms) {
            candidate.used = true;
            return Err(PairingError::Expired);
        }
        candidate.used = true;
        Ok(candidate.clone())
    }

    /// Seed a deterministic code for the smoke harness. Production callers use
    /// [`Self::issue`]; this only bypasses random generation, not redemption.
    pub fn seed(&mut self, mut code: PairingCode) {
        code.used = false;
        self.codes.push(code);
    }

    pub fn active(&self, now_ms: u64) -> usize {
        self.codes
            .iter()
            .filter(|code| !code.used && !code.expired(now_ms))
            .count()
    }

    pub fn prune(&mut self, now_ms: u64) {
        self.codes.retain(|code| code.used || !code.expired(now_ms));
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionToken(pub String);

impl SessionToken {
    pub fn issue() -> Self {
        let mut bytes = [0u8; 24];
        rand::rng().fill_bytes(&mut bytes);
        Self(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    pub fn verify(&self, candidate: &str) -> bool {
        constant_time_eq(self.0.as_bytes(), candidate.as_bytes())
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right) {
        difference |= a ^ b;
    }
    difference == 0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Waiting,
    Active,
    Closed,
}

impl SessionState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Waiting => "Waiting for viewer",
            Self::Active => "Streaming",
            Self::Closed => "Closed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub seq: u64,
    pub at_ms: u64,
    pub action: String,
    pub detail: String,
    pub prev: String,
    pub hash: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditChain {
    entries: Vec<AuditEntry>,
}

impl AuditChain {
    pub fn append(&mut self, now_ms: u64, action: &str, detail: &str) {
        let prev = self
            .entries
            .last()
            .map(|entry| entry.hash.clone())
            .unwrap_or_else(|| "genesis".to_string());
        let seq = self.entries.len() as u64 + 1;
        let mut hasher = Sha256::new();
        hasher.update(prev.as_bytes());
        hasher.update(seq.to_le_bytes());
        hasher.update(now_ms.to_le_bytes());
        hasher.update(action.as_bytes());
        hasher.update(detail.as_bytes());
        let hash = hex(&hasher.finalize());
        self.entries.push(AuditEntry {
            seq,
            at_ms: now_ms,
            action: action.to_string(),
            detail: detail.to_string(),
            prev,
            hash,
        });
    }

    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn verify(&self) -> bool {
        let mut prev = "genesis".to_string();
        for (index, entry) in self.entries.iter().enumerate() {
            if entry.seq != index as u64 + 1 || entry.prev != prev {
                return false;
            }
            let mut hasher = Sha256::new();
            hasher.update(prev.as_bytes());
            hasher.update(entry.seq.to_le_bytes());
            hasher.update(entry.at_ms.to_le_bytes());
            hasher.update(entry.action.as_bytes());
            hasher.update(entry.detail.as_bytes());
            if hex(&hasher.finalize()) != entry.hash {
                return false;
            }
            prev = entry.hash.clone();
        }
        true
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    HostOnly,
    Closed,
    Expired,
    InputDisabled,
    ClipboardDisabled,
    BadToken,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HostOnly => write!(f, "remote sessions are host-only"),
            Self::Closed => write!(f, "session is closed"),
            Self::Expired => write!(f, "session expired"),
            Self::InputDisabled => write!(f, "input is disabled for this session"),
            Self::ClipboardDisabled => write!(f, "clipboard is disabled for this session"),
            Self::BadToken => write!(f, "invalid session token"),
        }
    }
}

impl std::error::Error for SessionError {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteSession {
    pub id: String,
    pub token: SessionToken,
    pub pairing: PairingCode,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub policy: SessionPolicy,
    pub client: ClientCaps,
    pub tier: Tier,
    pub codec: Codec,
    pub state: SessionState,
    pub dtls_fingerprint: String,
    pub connected_at_ms: Option<u64>,
    pub audit: AuditChain,
}

impl RemoteSession {
    pub fn new(
        now_ms: u64,
        pairing: PairingCode,
        policy: SessionPolicy,
        client: ClientCaps,
        tier: Tier,
        codec: Codec,
        dtls_fingerprint: impl Into<String>,
    ) -> Self {
        let policy = policy.clamped();
        let id = format!("rs-{}", &SessionToken::issue().0[..12]);
        let mut session = Self {
            id,
            token: SessionToken::issue(),
            pairing,
            created_at_ms: now_ms,
            expires_at_ms: now_ms + policy.session_ttl_ms,
            policy,
            client,
            tier,
            codec,
            state: SessionState::Waiting,
            dtls_fingerprint: dtls_fingerprint.into(),
            connected_at_ms: None,
            audit: AuditChain::default(),
        };
        session.audit.append(
            now_ms,
            "session.open",
            &format!("{} {}", session.id, tier.short()),
        );
        session
            .audit
            .append(now_ms, "pairing.redeemed", &session.pairing.display());
        session
    }

    /// Only the host may start a session. Agents never receive this scope.
    pub fn authorize_host(subject: &str) -> Result<(), SessionError> {
        if subject == "host" {
            Ok(())
        } else {
            Err(SessionError::HostOnly)
        }
    }

    pub fn activate(&mut self, now_ms: u64) {
        if self.state != SessionState::Waiting {
            return;
        }
        self.state = SessionState::Active;
        self.connected_at_ms = Some(now_ms);
        self.audit
            .append(now_ms, "session.active", "dtls connected");
    }

    pub fn close(&mut self, now_ms: u64, reason: &str) {
        if self.state == SessionState::Closed {
            return;
        }
        self.state = SessionState::Closed;
        self.audit.append(now_ms, "session.closed", reason);
    }

    pub fn expired(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_at_ms
    }

    pub fn authorize_view(&self, now_ms: u64, token: &str) -> Result<(), SessionError> {
        if self.state == SessionState::Closed {
            return Err(SessionError::Closed);
        }
        if self.expired(now_ms) {
            return Err(SessionError::Expired);
        }
        if !self.token.verify(token) {
            return Err(SessionError::BadToken);
        }
        Ok(())
    }

    pub fn authorize_input(&mut self, now_ms: u64, token: &str) -> Result<(), SessionError> {
        self.authorize_view(now_ms, token)?;
        if !self.policy.allow_input {
            self.audit
                .append(now_ms, "input.denied", "policy disables input");
            return Err(SessionError::InputDisabled);
        }
        Ok(())
    }

    pub fn authorize_clipboard(&mut self, now_ms: u64, token: &str) -> Result<(), SessionError> {
        self.authorize_view(now_ms, token)?;
        if !self.policy.allow_clipboard {
            self.audit
                .append(now_ms, "clipboard.denied", "off by default");
            return Err(SessionError::ClipboardDisabled);
        }
        self.audit
            .append(now_ms, "clipboard.granted", "per-session toggle");
        Ok(())
    }

    pub fn toggle_clipboard(&mut self, now_ms: u64) {
        self.policy.allow_clipboard = !self.policy.allow_clipboard;
        self.audit.append(
            now_ms,
            "clipboard.toggle",
            if self.policy.allow_clipboard {
                "enabled"
            } else {
                "disabled"
            },
        );
    }

    pub fn toggle_input(&mut self, now_ms: u64) {
        self.policy.allow_input = !self.policy.allow_input;
        self.audit.append(
            now_ms,
            "input.toggle",
            if self.policy.allow_input {
                "enabled"
            } else {
                "disabled"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(now: u64) -> RemoteSession {
        let mut registry = PairingRegistry::new(DEFAULT_PAIRING_TTL_MS);
        let code = registry.issue(now);
        let redeemed = registry.redeem(now, &code.display()).unwrap();
        RemoteSession::new(
            now,
            redeemed,
            SessionPolicy::default(),
            ClientCaps::modern(),
            Tier::WebRtc,
            Codec::H264,
            "sha-256 AA:BB:CC",
        )
    }

    #[test]
    fn pairing_is_one_shot_and_expires() {
        let mut registry = PairingRegistry::new(1_000);
        let code = registry.issue(100);
        assert_eq!(code.display().len(), 9);
        assert!(code.display().contains('-'));
        assert_eq!(registry.active(100), 1);
        assert!(registry.redeem(200, &code.display().to_lowercase()).is_ok());
        assert_eq!(
            registry.redeem(200, &code.display()),
            Err(PairingError::Used)
        );
        let expired = registry.issue(300);
        assert_eq!(
            registry.redeem(1_400, &expired.display()),
            Err(PairingError::Expired)
        );
        assert_eq!(registry.redeem(100, "NOPE"), Err(PairingError::Invalid));
    }

    #[test]
    fn session_is_host_only_and_token_bound() {
        assert_eq!(RemoteSession::authorize_host("host"), Ok(()));
        assert_eq!(
            RemoteSession::authorize_host("agent-a"),
            Err(SessionError::HostOnly)
        );
        let mut session = session(1_000);
        let token = session.token.0.clone();
        assert_eq!(session.authorize_view(1_000, &token), Ok(()));
        assert_eq!(
            session.authorize_view(1_000, "bad"),
            Err(SessionError::BadToken)
        );
        assert_eq!(
            session.authorize_clipboard(1_000, &token),
            Err(SessionError::ClipboardDisabled)
        );
        assert!(session.audit.verify());
        session.toggle_clipboard(1_010);
        assert_eq!(session.authorize_clipboard(1_020, &token), Ok(()));
        assert!(session.audit.verify());
    }

    #[test]
    fn expiry_closes_the_view() {
        let mut registry = PairingRegistry::new(10_000);
        let code = registry.issue(0);
        let pairing = registry.redeem(0, &code.display()).unwrap();
        let policy = SessionPolicy {
            session_ttl_ms: 20_000,
            ..SessionPolicy::default()
        };
        let session = RemoteSession::new(
            1_000,
            pairing,
            policy,
            ClientCaps::modern(),
            Tier::WebRtc,
            Codec::H264,
            "sha-256",
        );
        let token = session.token.0.clone();
        assert_eq!(session.authorize_view(20_999, &token), Ok(()));
        assert_eq!(
            session.authorize_view(21_000, &token),
            Err(SessionError::Expired)
        );
    }

    #[test]
    fn audit_chain_detects_tampering() {
        let mut chain = AuditChain::default();
        chain.append(1, "session.open", "rs-1 webrtc");
        chain.append(2, "input.denied", "policy");
        chain.append(3, "session.closed", "client disconnect");
        assert_eq!(chain.len(), 3);
        assert!(chain.verify());
        let mut tampered = chain.clone();
        tampered.entries[1].detail = "edited".to_string();
        assert!(!tampered.verify());
    }

    #[test]
    fn state_transitions_are_monotonic() {
        let mut session = session(1_000);
        assert_eq!(session.state, SessionState::Waiting);
        session.activate(1_100);
        assert_eq!(session.state, SessionState::Active);
        assert_eq!(session.connected_at_ms, Some(1_100));
        session.close(1_200, "bye");
        assert_eq!(session.state, SessionState::Closed);
        session.activate(1_300);
        assert_eq!(session.state, SessionState::Closed);
        assert!(session.audit.verify());
    }
}
