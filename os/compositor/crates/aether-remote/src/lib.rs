//! Remote-display pipeline shared by the shell, the host service and tests.
//!
//! The crate is deliberately split into pure pieces:
//!
//! ```text
//! capture (DMA-BUF descriptor) ──> encoder (H.264, no queue)
//!        │                              │
//!        └── copy audit                 ├──> transport (str0m / fallback wire)
//!                                       └──> latency ledger
//!
//! signaling + pairing ──> session policy ──> virtual seat input
//! ```
//!
//! No compositor or Wayland types cross this boundary, so the whole pipeline is
//! testable on a development machine without a GPU or a display server. The
//! compositor binding only has to produce [`capture::CaptureFrame`] values with
//! exported DMA-BUF planes.

#![forbid(unsafe_code)]

pub mod capture;
pub mod encode;
pub mod ladder;
pub mod session;
pub mod signaling;

pub use capture::{
    CaptureFrame, CaptureSource, CopyAuditor, DamageRect, DmaBufFrame, FramePayload, PixelFormat,
};
pub use encode::{
    default_preferences, select_encoder, Codec, EncodedFrame, EncoderCaps, EncoderConfig,
    EncoderKind, H264Encoder, RateController,
};
pub use ladder::{latency_badge, select_codec, select_tier, ClientCaps, Tier};
pub use session::{
    AuditChain, PairingCode, PairingError, PairingRegistry, RemoteSession, SessionError,
    SessionPolicy, SessionState, SessionToken, REMOTE_HOST_SCOPE,
};
pub use signaling::{accept_hello, ClientHello, ServerWelcome, SignalMessage};
