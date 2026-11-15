//! Signaling messages exchanged before the media path starts.
//!
//! The host and browser exchange a small JSON protocol: hello (pairing code +
//! capabilities), welcome (session token + chosen tier), SDP offer/answer and
//! ICE candidates. The message model is independent of str0m so it can be
//! logged, replayed and tested without a network.

use serde::{Deserialize, Serialize};

use crate::encode::Codec;
use crate::ladder::{select_codec, select_tier, ClientCaps, Tier};
use crate::session::{PairingError, PairingRegistry, RemoteSession, SessionPolicy};

pub const SIGNAL_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub enum SignalError {
    Version { expected: u32, got: u32 },
    Pairing(PairingError),
    NoCodec,
    Json(String),
}

impl std::fmt::Display for SignalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Version { expected, got } => {
                write!(f, "signaling version {got}, expected {expected}")
            }
            Self::Pairing(error) => write!(f, "{error}"),
            Self::NoCodec => write!(f, "no common video codec"),
            Self::Json(error) => write!(f, "malformed signaling message: {error}"),
        }
    }
}

impl std::error::Error for SignalError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientHello {
    pub version: u32,
    pub pairing_code: String,
    pub client_name: String,
    pub udp_blocked: bool,
    pub caps: ClientCaps,
}

impl ClientHello {
    pub fn new(pairing_code: impl Into<String>, caps: ClientCaps) -> Self {
        Self {
            version: SIGNAL_VERSION,
            pairing_code: pairing_code.into(),
            client_name: "browser".to_string(),
            udp_blocked: false,
            caps,
        }
    }

    pub fn validate(&self) -> Result<(), SignalError> {
        if self.version != SIGNAL_VERSION {
            return Err(SignalError::Version {
                expected: SIGNAL_VERSION,
                got: self.version,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerWelcome {
    pub version: u32,
    pub session_id: String,
    pub token: String,
    pub expires_at_ms: u64,
    pub tier: Tier,
    pub codec: Codec,
    pub dtls_fingerprint: String,
    pub policy: SessionPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SignalMessage {
    Hello(ClientHello),
    Welcome(ServerWelcome),
    Offer { sdp: String },
    Answer { sdp: String },
    Candidate { candidate: String, mid: String },
    Bye { reason: String },
    Error { code: String, message: String },
}

impl SignalMessage {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn from_json(text: &str) -> Result<Self, SignalError> {
        serde_json::from_str(text).map_err(|error| SignalError::Json(error.to_string()))
    }
}

/// Redeem a hello and mint the session. `dtls_fingerprint` comes from the
/// host's str0m instance; `host_codecs` is the host preference list.
pub fn accept_hello(
    registry: &mut PairingRegistry,
    now_ms: u64,
    hello: &ClientHello,
    host_codecs: &[Codec],
    fingerprint: &str,
    policy: SessionPolicy,
) -> Result<(RemoteSession, ServerWelcome), SignalError> {
    hello.validate()?;
    let pairing = registry
        .redeem(now_ms, &hello.pairing_code)
        .map_err(SignalError::Pairing)?;
    let tier = select_tier(&hello.caps, hello.udp_blocked);
    let codec = select_codec(host_codecs, &hello.caps).ok_or(SignalError::NoCodec)?;
    let mut session = RemoteSession::new(
        now_ms,
        pairing,
        policy,
        hello.caps,
        tier,
        codec,
        fingerprint,
    );
    let welcome = ServerWelcome {
        version: SIGNAL_VERSION,
        session_id: session.id.clone(),
        token: session.token.0.clone(),
        expires_at_ms: session.expires_at_ms,
        tier,
        codec,
        dtls_fingerprint: session.dtls_fingerprint.clone(),
        policy: session.policy,
    };
    session
        .audit
        .append(now_ms, "signaling.welcome", tier.short());
    Ok((session, welcome))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{PairingError, DEFAULT_PAIRING_TTL_MS};

    #[test]
    fn hello_welcome_round_trip_selects_webrtc() {
        let mut registry = PairingRegistry::new(DEFAULT_PAIRING_TTL_MS);
        let code = registry.issue(1_000);
        let hello = ClientHello::new(code.display(), ClientCaps::modern());
        let (session, welcome) = accept_hello(
            &mut registry,
            1_000,
            &hello,
            &[Codec::H264],
            "sha-256 11:22",
            SessionPolicy::default(),
        )
        .expect("accepted");
        assert_eq!(welcome.tier, Tier::WebRtc);
        assert_eq!(welcome.codec, Codec::H264);
        assert_eq!(welcome.session_id, session.id);
        assert!(welcome.token.len() > 20);
        assert_eq!(welcome.dtls_fingerprint, "sha-256 11:22");
        assert!(!welcome.policy.allow_clipboard);
        assert!(session.audit.verify());

        let json = SignalMessage::Hello(hello).to_json();
        assert!(json.contains("\"type\":\"hello\""));
        assert!(matches!(
            SignalMessage::from_json(&json).unwrap(),
            SignalMessage::Hello(_)
        ));
    }

    #[test]
    fn rejects_old_versions_and_bad_codes() {
        let mut registry = PairingRegistry::new(10_000);
        let mut hello = ClientHello::new("NOPE", ClientCaps::modern());
        hello.version = 0;
        let error = accept_hello(
            &mut registry,
            0,
            &hello,
            &[Codec::H264],
            "sha-256",
            SessionPolicy::default(),
        )
        .unwrap_err();
        assert!(matches!(error, SignalError::Version { got: 0, .. }));

        hello.version = SIGNAL_VERSION;
        let error = accept_hello(
            &mut registry,
            0,
            &hello,
            &[Codec::H264],
            "sha-256",
            SessionPolicy::default(),
        )
        .unwrap_err();
        assert!(matches!(error, SignalError::Pairing(PairingError::Invalid)));
    }

    #[test]
    fn reports_no_common_codec() {
        let mut registry = PairingRegistry::new(10_000);
        let code = registry.issue(0);
        let caps = ClientCaps {
            h264: false,
            hevc: false,
            av1: false,
            ..ClientCaps::modern()
        };
        let hello = ClientHello::new(code.display(), caps);
        let error = accept_hello(
            &mut registry,
            0,
            &hello,
            &[Codec::H264],
            "sha-256",
            SessionPolicy::default(),
        )
        .unwrap_err();
        assert!(matches!(error, SignalError::NoCodec));
    }

    #[test]
    fn malformed_json_is_a_signal_error() {
        assert!(matches!(
            SignalMessage::from_json("{not json").unwrap_err(),
            SignalError::Json(_)
        ));
    }
}
