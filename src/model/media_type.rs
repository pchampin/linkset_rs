use std::sync::LazyLock;

use regex::Regex;

use crate::error::MediaTypeError;

/// A plausible media type string, e.g. `text/html` or `application/linkset+json`.
///
/// Validated per [RFC 7231 §3.1.1.1](https://www.rfc-editor.org/rfc/rfc7231#section-3.1.1.1):
/// the `type` and `subtype` portions must be non-empty
/// [RFC 7230 §3.2.6](https://www.rfc-editor.org/rfc/rfc7230#section-3.2.6) tokens.
/// Parameters (after `;`) are accepted without further validation.
///
/// No check is made that the type is registered with IANA.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MediaType(String);

impl MediaType {
    /// Parse and validate `s` as a plausible media type.
    ///
    /// Returns [`MediaTypeError`] if the `type/subtype` portion is malformed.
    pub fn new(s: impl Into<String>) -> Result<Self, MediaTypeError> {
        let s = s.into();
        if MEDIA_TYPE_RE.is_match(&s) {
            Ok(MediaType(s))
        } else {
            Err(MediaTypeError(s))
        }
    }

    /// Wrap a pre-validated media-type string without re-checking.
    pub fn new_unchecked(s: String) -> Self {
        MediaType(s)
    }

    /// Return the media type as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Debug for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MediaType({:?})", self.0)
    }
}

impl std::ops::Deref for MediaType {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for MediaType {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for MediaType {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

/// Matches a plausible media type per [RFC 7231 §3.1.1.1](https://www.rfc-editor.org/rfc/rfc7231#section-3.1.1.1).
///
/// Built from named pieces for readability:
/// - `tchar`: [RFC 7230 §3.2.6](https://www.rfc-editor.org/rfc/rfc7230#section-3.2.6) token character
/// - `token`: one or more tchars
/// - `quoted_string`: `"..."` with backslash escapes
/// - `param`: `OWS ";" OWS token "=" (token / quoted-string)`
static MEDIA_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    let tchar = r#"[!#$%&'*+.^_`|~0-9A-Za-z-]"#;
    let token = format!("{tchar}+");
    let quoted_string = r#""[^"\\]*(?:\\.[^"\\]*)*""#;
    let param = format!(r#"(?:\s*;\s*{token}=(?:{token}|{quoted_string}))"#);
    Regex::new(&format!("^{token}/{token}(?:{param})*$")).unwrap()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_simple() {
        assert!(MediaType::new("text/html").is_ok());
        assert!(MediaType::new("application/linkset+json").is_ok());
    }

    #[test]
    fn valid_with_params() {
        assert!(MediaType::new("text/html; charset=utf-8").is_ok());
        assert!(MediaType::new("application/json;q=0.9").is_ok());
    }

    #[test]
    fn invalid() {
        assert!(MediaType::new("text").is_err());
        assert!(MediaType::new("/html").is_err());
        assert!(MediaType::new("text/").is_err());
        assert!(MediaType::new("te xt/html").is_err());
    }
}
