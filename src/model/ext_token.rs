use crate::error::ExtensionError;

/// Well-known plain (no `*`) link parameter names per
/// [RFC 8288 §3](https://www.rfc-editor.org/rfc/rfc8288#section-3).
pub const WELL_KNOWN_KEYS: &[&str] =
    &["rel", "rev", "anchor", "type", "title", "hreflang", "media"];

/// Well-known `*`-suffixed link parameter names per
/// [RFC 8187](https://www.rfc-editor.org/rfc/rfc8187).
pub const WELL_KNOWN_I18N_KEYS: &[&str] = &["title*"];

/// Return `true` if `c` is an [RFC 7230 §3.2.6](https://www.rfc-editor.org/rfc/rfc7230#section-3.2.6) `tchar`.
fn is_tchar(c: char) -> bool {
    matches!(
        c,
        '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' | '-' | '.' | '^' | '_' | '`' | '|' | '~'
    ) || c.is_ascii_alphanumeric()
}

/// Return `true` if `s` is a non-empty [RFC 7230 §3.2.6](https://www.rfc-editor.org/rfc/rfc7230#section-3.2.6) `token`.
pub fn is_valid_token(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_tchar)
}

/// Validate an extension attribute key, distinguishing plain (`exts`) from i18n (`exts_i18n`) keys.
pub(super) fn validate_ext_attribute(key: &str, i18n: bool) -> Result<(), ExtensionError> {
    if !is_valid_token(key) {
        return Err(ExtensionError::InvalidKey(key.to_string()));
    }
    if i18n {
        if !key.ends_with('*') {
            return Err(ExtensionError::StarSuffixRequired);
        }
        if WELL_KNOWN_I18N_KEYS.contains(&key) {
            return Err(ExtensionError::ReservedKey(key.to_string()));
        }
    } else {
        if key.ends_with('*') {
            return Err(ExtensionError::StarSuffixForbidden);
        }
        if WELL_KNOWN_KEYS.contains(&key) {
            return Err(ExtensionError::ReservedKey(key.to_string()));
        }
    }
    Ok(())
}
