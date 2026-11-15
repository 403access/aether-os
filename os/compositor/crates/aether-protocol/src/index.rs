//! File-index query protocol.
//!
//! The daemon reads one [`IndexRequest`] per line and answers with one
//! [`IndexResponse`]. The same shapes are used by the palette worker and the
//! MCP file-search tool.

use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Queries default to the same page size as the daemon.
pub const DEFAULT_LIMIT: usize = 8;
/// Upper bound accepted by the daemon.
pub const MAX_LIMIT: usize = 20_000;

fn default_limit() -> usize {
    DEFAULT_LIMIT
}

/// One query. `limit` is clamped to the daemon's accepted range on construction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexRequest {
    #[serde(default)]
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

impl IndexRequest {
    pub fn new(query: impl Into<String>, limit: usize) -> Self {
        Self {
            query: query.into(),
            limit: limit.clamp(1, MAX_LIMIT),
        }
    }

    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("index request is always serializable")
    }

    pub fn parse_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        serde_json::from_str(trimmed).ok()
    }
}

/// One ranked result, already shaped for the palette and MCP clients.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexHit {
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    pub path: String,
}

/// The daemon's response to one [`IndexRequest`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexResponse {
    #[serde(default)]
    pub results: Vec<IndexHit>,
}

impl IndexResponse {
    pub fn new(results: Vec<IndexHit>) -> Self {
        Self { results }
    }

    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("index response is always serializable")
    }

    pub fn parse_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        serde_json::from_str(trimmed).ok()
    }
}

/// Run one query round trip against the index socket.
pub fn query(
    socket: &Path,
    request: &IndexRequest,
    timeout: Duration,
) -> std::io::Result<IndexResponse> {
    let stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let mut writer = stream.try_clone()?;
    super::line::write_json_line(&mut writer, request)?;
    let mut reader = BufReader::new(stream);
    let response = super::line::read_json_line(&mut reader)?;
    response.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "index daemon closed the stream",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_clamp_the_limit_and_round_trip() {
        let request = IndexRequest::new("notes", 0);
        assert_eq!(request.limit, 1);
        assert_eq!(
            IndexRequest::parse_line(&request.to_line()),
            Some(request.clone())
        );

        let too_large = IndexRequest::new("notes", MAX_LIMIT + 1);
        assert_eq!(too_large.limit, MAX_LIMIT);
    }

    #[test]
    fn requests_default_to_the_daemon_page_size() {
        let request = IndexRequest::parse_line(r#"{"query":"notes"}"#).unwrap();
        assert_eq!(request.limit, DEFAULT_LIMIT);
    }

    #[test]
    fn responses_match_the_documented_shape() {
        let line = r#"{"results":[{"title":"notes.md","subtitle":"/home/u/notes  ·  md","path":"/home/u/notes.md"}]}"#;
        let response = IndexResponse::parse_line(line).unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].title, "notes.md");
        assert_eq!(response.results[0].path, "/home/u/notes.md");
        assert_eq!(
            IndexResponse::parse_line(&response.to_line()),
            Some(response)
        );
    }
}
