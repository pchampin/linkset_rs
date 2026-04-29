use std::sync::LazyLock;

use regex::Regex;
use sophia_iri::uri::Uri;

use crate::error::RelTypeError;

/// A link relation type, as defined in
/// [RFC 8288 §2.1](https://www.rfc-editor.org/rfc/rfc8288#section-2.1).
///
/// A relation type is either:
/// - a **registered** name ([`RelType::Reg`]): a short token
///   (e.g. `"item"`, `"describedby"`); or
/// - an **extension** URI ([`RelType::Ext`]): an absolute URI.
///
/// No check is made that a registered name is actually listed in the IANA registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RelType {
    /// A registered relation type name.
    Reg(String),
    /// An extension relation type expressed as an absolute URI.
    Ext(Uri<String>),
}

impl RelType {
    /// Construct a relation type, deciding the variant based on the the form of `s`,
    /// returning `None` if no variant can be matched.
    ///
    /// If `s` matches the syntax of a registered relationship,
    /// no check is made that the name is actually registered with IANA.
    pub fn new(s: impl Into<String>) -> Option<Self> {
        let s = s.into();
        if REL_TYPE_RE.is_match(&s) {
            Some(RelType::Reg(s))
        } else if let Ok(uri) = Uri::new(s) {
            Some(RelType::Ext(uri))
        } else {
            None
        }
    }

    /// Construct a registered relation type, validating that `s` is a
    /// [RFC 7230 §3.2.6](https://www.rfc-editor.org/rfc/rfc7230#section-3.2.6) token.
    ///
    /// No check is made that the name is actually registered with IANA.
    pub fn new_reg(s: impl Into<String>) -> Result<Self, RelTypeError> {
        let s = s.into();
        if REL_TYPE_RE.is_match(&s) {
            Ok(RelType::Reg(s))
        } else {
            Err(RelTypeError(s))
        }
    }

    /// Construct a registered relation type without validating the token format.
    pub fn new_reg_unchecked(s: impl Into<String>) -> Self {
        RelType::Reg(s.into())
    }

    /// Construct an extension relation type from an already-validated [`Uri`].
    pub fn new_ext(uri: Uri<String>) -> Self {
        RelType::Ext(uri)
    }

    /// Return the relation type as a string slice.
    pub fn as_str(&self) -> &str {
        match self {
            RelType::Reg(s) => s.as_str(),
            RelType::Ext(uri) => uri.as_str(),
        }
    }
}

impl std::fmt::Display for RelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::ops::Deref for RelType {
    type Target = str;
    fn deref(&self) -> &str {
        match self {
            RelType::Reg(s) => s,
            RelType::Ext(uri) => uri,
        }
    }
}

impl AsRef<str> for RelType {
    fn as_ref(&self) -> &str {
        self
    }
}

impl PartialEq<str> for RelType {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

/// Matches a plausible relation type per [RFC 8288 §3.3](https://www.rfc-editor.org/rfc/rfc8288.html#section-3.3).
static REL_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[a-z][a-z0-9.-]*$").unwrap());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_reg_valid() {
        let r = RelType::new_reg("item").unwrap();
        assert_eq!(r.as_str(), "item");
        assert!(matches!(r, RelType::Reg(_)));
    }

    #[test]
    fn new_reg_invalid() {
        assert!(RelType::new_reg("not valid").is_err());
        assert!(RelType::new_reg("").is_err());
    }

    #[test]
    fn new_ext() {
        let uri = Uri::new("https://example.com/rel/owns".into()).unwrap();
        let r = RelType::new_ext(uri);
        assert_eq!(r.as_str(), "https://example.com/rel/owns");
        assert!(matches!(r, RelType::Ext(_)));
    }
}
