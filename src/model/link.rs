use indexmap::IndexMap;

use super::ext_token::validate_ext_attribute;
use super::media_type::MediaType;
use super::rel_type::RelType;

use sophia_bcp47::LanguageTag;
use sophia_iri::uri::Uri;

/// A single typed link from an anchor context to a target.
///
/// Corresponds to one link-value in [RFC 8288 §3](https://www.rfc-editor.org/rfc/rfc8288#section-3).
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    /// The link target URI (`href` in JSON, the bracketed URI in the text format).
    target: Uri<String>,
    /// The relation type ([RFC 8288 §2.1](https://www.rfc-editor.org/rfc/rfc8288#section-2.1)) of this link.
    rel: RelType,
    /// Optional hint for the media type of the target (`type` in both formats).
    pub type_: Option<MediaType>,
    /// Optional human-readable label (`title`).
    pub title: Option<String>,
    /// Optional internationalised label (`title*`,
    /// [RFC 8187](https://www.rfc-editor.org/rfc/rfc8187)).
    /// (`i18n` is short for "internationalization")
    pub title_i18n: Option<I18nString>,
    /// Zero or more language hints for the target (`hreflang`).
    ///
    /// Multiple values are permitted per
    /// [RFC 8288 §3](https://www.rfc-editor.org/rfc/rfc8288#section-3).
    pub hreflang: Vec<LanguageTag<String>>,
    /// Optional media query hint (`media`).
    pub media: Option<String>,
    /// Extension attributes with string values (private; access via [`Link::get_ext`] etc.).
    exts: IndexMap<String, Vec<String>>,
    /// Extension attributes with i18n values (private; access via [`Link::get_ext_i18n`] etc.).
    exts_i18n: IndexMap<String, Vec<I18nString>>,
}

impl Link {
    /// Create a new `Link` with the required fields, validating `target` as a URI.
    pub fn new(target: Uri<String>, rel: RelType) -> Self {
        Link {
            target,
            rel,
            type_: None,
            title: None,
            title_i18n: None,
            hreflang: Vec::new(),
            media: None,
            exts: IndexMap::new(),
            exts_i18n: IndexMap::new(),
        }
    }

    /// This link's target
    pub fn target(&self) -> &Uri<String> {
        &self.target
    }

    /// This link's rel type
    pub fn rel(&self) -> &RelType {
        &self.rel
    }

    /// Return the values for an extension attribute, or `None` if absent.
    ///
    /// `key` is matched case-insensitively per
    /// [RFC 7230 §3.2](https://www.rfc-editor.org/rfc/rfc7230#section-3.2).
    pub fn get_ext(&self, key: &str) -> Option<&Vec<String>> {
        self.exts.get(&key.to_ascii_lowercase()[..])
    }

    /// Return a mutable reference to the values for an extension attribute.
    ///
    /// **Note:** the returned `Vec` must not be made empty; use [`del_ext`](Self::del_ext)
    /// to remove the parameter entirely.
    ///
    /// `key` is matched case-insensitively.
    pub fn get_ext_mut(&mut self, key: &str) -> Option<&mut Vec<String>> {
        self.exts.get_mut(&key.to_ascii_lowercase()[..])
    }

    /// Set an extension attribute with string values, returning the previous
    /// values if the parameter was already present.
    ///
    /// `key` must be a valid [RFC 7230 §3.2.6](https://www.rfc-editor.org/rfc/rfc7230#section-3.2.6)
    /// token, must not end with `'*'`, and must not be a well-known attribute name.
    /// Panics if any of these constraints are violated.
    ///
    /// **Note:** if `values` is empty, this method will behave as [`del_ext`](Link::del_ext).
    pub fn set_ext(&mut self, key: &str, values: Vec<String>) -> Option<Vec<String>> {
        if values.is_empty() {
            return self.del_ext(key);
        }
        let key = key.to_string().to_ascii_lowercase();
        validate_ext_attribute(&key, false).unwrap_or_else(|e| panic!("{e}"));
        self.exts.insert(key, values)
    }

    /// Remove an extension attribute, returning its values if it was present.
    ///
    /// `key` is matched case-insensitively.
    pub fn del_ext(&mut self, key: &str) -> Option<Vec<String>> {
        self.exts.shift_remove(&key.to_ascii_lowercase()[..])
    }

    /// Iterate over all plain extension attributes as `(key, values)` pairs.
    pub fn iter_ext(&self) -> impl Iterator<Item = (&str, &[String])> {
        self.exts.iter().map(|(k, v)| (k.as_str(), v.as_slice()))
    }

    /// Return the i18n values for an extension attribute (`param*` syntax),
    /// or `None` if absent.
    ///
    /// `key` must include the trailing `'*'` and is matched case-insensitively.
    pub fn get_ext_i18n(&self, key: &str) -> Option<&Vec<I18nString>> {
        self.exts_i18n.get(&key.to_ascii_lowercase()[..])
    }

    /// Return a mutable reference to the i18n values for an extension attribute.
    ///
    /// **Note:** the returned `Vec` must not be made empty; use [`del_ext_i18n`](Self::del_ext_i18n)
    /// to remove the parameter entirely.
    ///
    /// `key` must include the trailing `'*'` and is matched case-insensitively.
    pub fn get_ext_i18n_mut(&mut self, key: &str) -> Option<&mut Vec<I18nString>> {
        self.exts_i18n.get_mut(&key.to_ascii_lowercase()[..])
    }

    /// Set an extension attribute with i18n values, returning the previous
    /// values if the parameter was already present.
    ///
    /// `key` must be a valid [RFC 7230 §3.2.6](https://www.rfc-editor.org/rfc/rfc7230#section-3.2.6)
    /// token ending with `'*'`, and must not be a well-known i18n attribute name.
    /// Panics if any of these constraints are violated.
    ///
    /// **Note:** if `values` is empty, this method will behave as [`del_ext_i18n`](Link::del_ext_i18n).
    pub fn set_ext_i18n(&mut self, key: &str, values: Vec<I18nString>) -> Option<Vec<I18nString>> {
        if values.is_empty() {
            return self.del_ext_i18n(key);
        }
        let key = key.to_string().to_ascii_lowercase();
        validate_ext_attribute(&key, true).unwrap_or_else(|e| panic!("{e}"));
        self.exts_i18n.insert(key, values)
    }

    /// Remove an i18n extension attribute, returning its values if it was present.
    ///
    /// `key` must include the trailing `'*'` and is matched case-insensitively.
    pub fn del_ext_i18n(&mut self, key: &str) -> Option<Vec<I18nString>> {
        self.exts_i18n.shift_remove(&key.to_ascii_lowercase()[..])
    }

    /// Iterate over all i18n extension attributes as `(key, values)` pairs.
    pub fn iter_ext_i18n(&self) -> impl Iterator<Item = (&str, &[I18nString])> {
        self.exts_i18n
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_slice()))
    }
}

pub type I18nString = sophia_bcp47::I18nString<String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_with_all_fields() {
        let mut link = Link::new(
            Uri::new_unchecked("https://example.com/resource".into()),
            RelType::new_reg_unchecked("describedby"),
        );
        link.type_ = Some(MediaType::new_unchecked("text/html".to_string()));
        link.title = Some("Example".to_string());
        link.hreflang = vec![
            LanguageTag::new_unchecked("en".to_string()),
            LanguageTag::new_unchecked("fr".to_string()),
        ];
        link.title_i18n = Some(I18nString {
            language: LanguageTag::new_unchecked("en".to_string()),
            value: "Example resource".to_string(),
        });
        assert_eq!(link.hreflang.len(), 2);
        assert!(link.title_i18n.is_some());
    }

    #[test]
    fn ext_set_get_del() {
        let mut link = Link::new(
            Uri::new_unchecked("https://example.com/".into()),
            RelType::new_reg_unchecked("item"),
        );
        assert!(link.set_ext("foo", vec!["bar".to_string()]).is_none());
        assert_eq!(link.get_ext("FOO").unwrap(), &["bar"]);
        let prev = link.set_ext("foo", vec!["baz".to_string()]);
        assert_eq!(prev.unwrap(), ["bar"]);
        assert_eq!(link.del_ext("foo").unwrap(), ["baz"]);
        assert!(link.get_ext("foo").is_none());
    }

    #[test]
    fn ext_i18n_set_get_del() {
        let mut link = Link::new(
            Uri::new_unchecked("https://example.com/".into()),
            RelType::new_reg_unchecked("item"),
        );
        let v = I18nString {
            language: LanguageTag::new_unchecked("fr".to_string()),
            value: "Bonjour".to_string(),
        };
        assert!(link.set_ext_i18n("label*", vec![v.clone()]).is_none());
        assert_eq!(
            link.get_ext_i18n("LABEL*").unwrap(),
            std::slice::from_ref(&v)
        );
        assert_eq!(link.del_ext_i18n("label*").unwrap(), [v]);
        assert!(link.get_ext_i18n("label*").is_none());
    }

    #[test]
    fn ext_set_get_set_empty() {
        let mut link = Link::new(
            Uri::new_unchecked("https://example.com/".into()),
            RelType::new_reg_unchecked("item"),
        );
        assert!(link.set_ext("foo", vec!["bar".to_string()]).is_none());
        assert_eq!(link.get_ext("FOO").unwrap(), &["bar"]);
        let prev = link.set_ext("foo", vec!["baz".to_string()]);
        assert_eq!(prev.unwrap(), ["bar"]);
        assert_eq!(link.set_ext("foo", vec![]).unwrap(), ["baz"]);
        assert!(link.get_ext("foo").is_none());
    }

    #[test]
    fn ext_i18n_set_get_set_empty() {
        let mut link = Link::new(
            Uri::new_unchecked("https://example.com/".into()),
            RelType::new_reg_unchecked("item"),
        );
        let v = I18nString {
            language: LanguageTag::new_unchecked("fr".to_string()),
            value: "Bonjour".to_string(),
        };
        assert!(link.set_ext_i18n("label*", vec![v.clone()]).is_none());
        assert_eq!(
            link.get_ext_i18n("LABEL*").unwrap(),
            std::slice::from_ref(&v)
        );
        assert_eq!(link.set_ext_i18n("label*", vec![]).unwrap(), [v]);
        assert!(link.get_ext_i18n("label*").is_none());
    }

    #[test]
    #[should_panic]
    fn set_ext_rejects_reserved_key() {
        let mut link = Link::new(
            Uri::new_unchecked("https://example.com/".into()),
            RelType::new_reg_unchecked("item"),
        );
        link.set_ext("title", vec!["x".to_string()]);
    }

    #[test]
    #[should_panic]
    fn set_ext_rejects_star_suffix() {
        let mut link = Link::new(
            Uri::new_unchecked("https://example.com/".into()),
            RelType::new_reg_unchecked("item"),
        );
        link.set_ext("foo*", vec!["x".to_string()]);
    }
}
