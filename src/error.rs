//! I define the error type [`LinksetError`] for [`Linkset`](crate::Linkset) processing.

use sophia_iri::uri::Uri;
use thiserror::Error;

use crate::model::RelType;

/// Error that can occur when handling LinkSets
#[derive(Debug, Error, PartialEq)]
pub enum LinksetError {
    /// A link set must contain at least one context
    #[error("empty linkset")]
    Empty,
    /// A link set can not contain two contexts with the same anchor
    #[error("duplicate link: <{2}>; rel=\"{1}\"; anchor=\"{0}\"")]
    DuplicateLink(Uri<String>, RelType, Uri<String>),
    /// A JSON document is structurally invalid or contains an invalid value.
    #[error("JSON: {0}")]
    Json(String),
    /// A text (`application/linkset`) document is syntactically or semantically invalid.
    #[error("text: {0}")]
    Text(String),
}

impl From<serde_json::Error> for LinksetError {
    fn from(value: serde_json::Error) -> Self {
        LinksetError::Json(format!("{value}"))
    }
}

/// Error returned when a string is not a plausible media type.
#[derive(Debug, Error, PartialEq)]
#[error("not a plausible media type: {0:?}")]
pub struct MediaTypeError(pub String);

/// Error returned when a string is not a valid registered relation type.
#[derive(Debug, Error, PartialEq)]
#[error("not a plausible registered relation type (must be a RFC 7230 token): {0:?}")]
pub struct RelTypeError(pub String);

/// Errors that can occur when accessing extension link attributes.
#[derive(Debug, Error, PartialEq)]
pub enum ExtensionError {
    /// The key is not a valid [RFC 7230 §3.2.6](https://www.rfc-editor.org/rfc/rfc7230#section-3.2.6) token.
    #[error("invalid attribute key (must be a valid RFC 7230 token): {0:?}")]
    InvalidKey(String),
    /// The key names a well-known link attribute that has a dedicated field on [`Link`](crate::model::Link).
    /// Use the dedicated field instead.
    #[error("reserved attribute key: {0:?}")]
    ReservedKey(String),
    /// A key passed to an `ext_i18n` method must end with `'*'`.
    #[error("key for internationalised extension attributes must end with '*'")]
    StarSuffixRequired,
    /// A key passed to an `ext` method must not end with `'*'`.
    #[error("key for plain extension attributes must not end with '*'")]
    StarSuffixForbidden,
}
