//! I provide implementation for parsng and serializing [`Linkset`]s
//! in the text format defined by [RFC 9264], `application/linkset`.
//!
//! See:
//! * [`Linkset::from_text_slice`]
//! * [`Linkset::from_text_str`]
//! * [`Linkset::to_text_string`]
//! * [`Linkset::to_text_vec`]
//! * [`Linkset::to_text_writer`]
//!
//! [RFC 9264]: https://www.rfc-editor.org/rfc/rfc9264.html

use std::{borrow::Cow, io};

use sophia_bcp47::LanguageTag;
use sophia_iri::{
    resolve::BaseIri,
    uri::{InvalidUri, Uri, UriRef},
};
use winnow::{
    ModalResult, Parser,
    ascii::multispace0,
    combinator::{alt, delimited, opt, preceded, repeat, separated},
    error::{ContextError, ParseError},
    token::{any, take_while},
};

use crate::{
    Linkset,
    error::LinksetError,
    model::{I18nString, Link, LinkContext, MediaType, RelType, ext_token},
    relativizer::UriRelativizer,
};

impl Linkset {
    /// Create a [`Linkset`] from a string in `application/linkset` format
    /// as defined by [RFC 9264 §4.1](https://www.rfc-editor.org/rfc/rfc9264.html#name-link-set-document-format-ap).
    pub fn from_text_str(s: &str, base: Option<Uri<&str>>) -> Result<Linkset, LinksetError> {
        Linkset::from_text_slice(s.as_bytes(), base)
    }

    /// Create a [`Linkset`] from bytes in `application/linkset` format
    /// as defined by [RFC 9264 §4.1](https://www.rfc-editor.org/rfc/rfc9264.html#name-link-set-document-format-ap).
    pub fn from_text_slice(slice: &[u8], base: Option<Uri<&str>>) -> Result<Linkset, LinksetError> {
        let base = base.map(|b| b.map_unchecked(String::from).into_iri().to_base());
        let entries = parse_all_entries(slice).map_err(|e| LinksetError::Text(e.to_string()))?;
        entries_to_linkset(entries, base.as_ref())
    }

    /// Write this [`Linkset`] as `application/linkset` to a stream
    /// as defined by [RFC 9264 §4.1](https://www.rfc-editor.org/rfc/rfc9264.html#name-link-set-document-format-ap).
    ///
    /// With `pretty = false` the output is a single line suitable for use as an HTTP
    /// [`Link`](https://www.rfc-editor.org/rfc/rfc8288#section-3) header value.
    /// With `pretty = true`, newlines and indentations will be used.
    ///
    /// If `base` is not None, anchors and targets are relativized against it.
    pub fn to_text_writer(
        &self,
        mut writer: impl io::Write,
        pretty: bool,
        base: Option<Uri<&str>>,
    ) -> io::Result<()> {
        let rel = UriRelativizer::new(base);
        let mut first = true;
        for ctx in self {
            for link in ctx {
                if !first {
                    writer.write_all(if pretty { b",\n" } else { b", " })?;
                }
                first = false;
                let anchor = rel.relativize(ctx.anchor().as_ref());
                write_link_value(&mut writer, link, anchor, pretty, &rel)?;
            }
        }
        Ok(())
    }

    /// Serialize this [`Linkset`] as `application/linkset` bytes
    /// as defined by [RFC 9264 §4.1](https://www.rfc-editor.org/rfc/rfc9264.html#name-link-set-document-format-ap).
    ///
    /// With `pretty = false` the output is a single line suitable for use as an HTTP
    /// [`Link`](https://www.rfc-editor.org/rfc/rfc8288#section-3) header value.
    /// With `pretty = true`, newlines and indentations will be used.
    ///
    /// If `base` is not None, anchors and targets are relativized against it.
    pub fn to_text_vec(&self, pretty: bool, base: Option<Uri<&str>>) -> Vec<u8> {
        let mut buf = Vec::new();
        self.to_text_writer(&mut buf, pretty, base)
            .expect("writing to Vec<u8> is infallible");
        buf
    }

    /// Serialize this [`Linkset`] as an `application/linkset` string
    /// as defined by [RFC 9264 §4.1](https://www.rfc-editor.org/rfc/rfc9264.html#name-link-set-document-format-ap).
    ///
    /// With `pretty = false` the output is a single line suitable for use as an HTTP
    /// [`Link`](https://www.rfc-editor.org/rfc/rfc8288#section-3) header value.
    /// With `pretty = true`, newlines and indentations will be used.
    ///
    /// If `base` is not None, anchors and targets are relativized against it.
    pub fn to_text_string(&self, pretty: bool, base: Option<Uri<&str>>) -> String {
        // Safe: to_text_writer only emits ASCII bytes.
        unsafe { String::from_utf8_unchecked(self.to_text_vec(pretty, base)) }
    }
}

// ── Raw parsed structures ────────────────────────────────────────────────────

struct Entry {
    target: String,
    params: Vec<(String, ParamValue)>,
}

enum ParamValue {
    Plain(String),
    Ext {
        charset: String,
        language: Option<String>,
        encoded: String,
    },
}

// ── Winnow parser ────────────────────────────────────────────────────────────

/// Return true for RFC 7230 §3.2.6 tchar.
fn is_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

/// Convert an ASCII-validated byte slice to a `String`.
///
/// Panics if called with non-ASCII bytes — the parser predicates guarantee this never happens.
fn ascii_to_string(b: &[u8]) -> String {
    std::str::from_utf8(b)
        .expect("parser invariant: only ASCII bytes reach here")
        .to_string()
}

fn token<'i>(input: &mut &'i [u8]) -> ModalResult<&'i [u8]> {
    take_while(1.., is_tchar).parse_next(input)
}

fn uri_in_brackets<'i>(input: &mut &'i [u8]) -> ModalResult<&'i [u8]> {
    delimited(
        b'<',
        take_while(0.., |b: u8| b != b'>' && b.is_ascii()),
        b'>',
    )
    .parse_next(input)
}

fn quoted_string(input: &mut &[u8]) -> ModalResult<String> {
    delimited(
        b'"',
        repeat(
            0..,
            alt((
                // escaped character — verify ASCII so we don't accept e.g. \ü
                preceded(b'\\', any.verify(|b: &u8| b.is_ascii()))
                    .map(|b: u8| String::from(b as char)),
                // run of printable ASCII characters (no quote, no backslash)
                take_while(1.., |b: u8| b != b'"' && b != b'\\' && b.is_ascii())
                    .map(|bytes: &[u8]| ascii_to_string(bytes)),
            )),
        ),
        b'"',
    )
    .map(|parts: Vec<String>| parts.concat())
    .parse_next(input)
}

/// Parse an [RFC 8187](https://www.rfc-editor.org/rfc/rfc8187) `ext-value`:
/// `charset "'" [ language ] "'" value-chars`.
fn ext_value(input: &mut &[u8]) -> ModalResult<ParamValue> {
    // charset = mime-charset, which excludes "'" (the ext-value delimiter)
    let charset: &[u8] = take_while(1.., |b: u8| is_tchar(b) && b != b'\'').parse_next(input)?;
    b'\''.parse_next(input)?;
    let language: Option<&[u8]> =
        opt(take_while(1.., |b: u8| b != b'\'' && b.is_ascii())).parse_next(input)?;
    b'\''.parse_next(input)?;
    let encoded: &[u8] = take_while(0.., |b: u8| {
        b.is_ascii() && !b.is_ascii_whitespace() && b != b';' && b != b','
    })
    .parse_next(input)?;
    Ok(ParamValue::Ext {
        charset: ascii_to_string(charset),
        language: language.map(ascii_to_string),
        encoded: ascii_to_string(encoded),
    })
}

/// Parse a single `token "=" param-value` pair.
fn link_param(input: &mut &[u8]) -> ModalResult<(String, ParamValue)> {
    let name: &[u8] = token.parse_next(input)?;
    b'='.parse_next(input)?;
    let name_str = ascii_to_string(name);
    let value = if name_str.ends_with('*') {
        ext_value.parse_next(input)?
    } else {
        alt((
            quoted_string.map(ParamValue::Plain),
            token.map(|b: &[u8]| ParamValue::Plain(ascii_to_string(b))),
        ))
        .parse_next(input)?
    };
    Ok((name_str, value))
}

/// Parse one `link-value` (`"<" URI ">" *( OWS ";" OWS link-param )`).
fn link_value(input: &mut &[u8]) -> ModalResult<Entry> {
    let target: &[u8] = uri_in_brackets.parse_next(input)?;
    let params: Vec<(String, ParamValue)> =
        repeat(0.., preceded((multispace0, b';', multispace0), link_param)).parse_next(input)?;
    Ok(Entry {
        target: ascii_to_string(target),
        params,
    })
}

/// Parse a comma-separated list of `link-value`s with optional surrounding whitespace.
fn all_link_values(input: &mut &[u8]) -> ModalResult<Vec<Entry>> {
    delimited(
        multispace0,
        separated(0.., link_value, (multispace0, b',', multispace0)),
        multispace0,
    )
    .parse_next(input)
}

fn parse_all_entries(s: &[u8]) -> Result<Vec<Entry>, ParseError<&[u8], ContextError>> {
    all_link_values.parse(s)
}

// ── Entry → LinkContext conversion ───────────────────────────────────────────

fn text_err(msg: impl Into<String>) -> LinksetError {
    LinksetError::Text(msg.into())
}

/// Resolve a URI-reference string against an optional base.
///
/// Precondition: base is known to be a URI (not just any IRI).
fn resolve(base: Option<&BaseIri<String>>, txt: &str) -> Result<Uri<String>, InvalidUri> {
    if let Some(base) = base {
        let uri_ref = UriRef::new(txt.to_string())?.into_iri_ref();
        Ok(base.resolve(uri_ref).to_uri_unchecked())
    } else {
        Uri::new(txt.to_string())
    }
}

fn plain_value(value: ParamValue) -> Result<String, LinksetError> {
    match value {
        ParamValue::Plain(s) => Ok(s),
        ParamValue::Ext { .. } => Err(text_err("unexpected ext-value for plain attribute")),
    }
}

/// Decode an [RFC 8187](https://www.rfc-editor.org/rfc/rfc8187) ext-value into an [`I18nString`].
///
/// Only US-ASCII and UTF-8 charsets are supported.
fn decode_ext_value(value: ParamValue) -> Result<I18nString, LinksetError> {
    let (charset, language, encoded) = match value {
        ParamValue::Ext {
            charset,
            language,
            encoded,
        } => (charset, language, encoded),
        ParamValue::Plain(_) => return Err(text_err("expected ext-value for i18n attribute")),
    };

    let ascii = if charset.eq_ignore_ascii_case("US-ASCII") {
        true
    } else if charset.eq_ignore_ascii_case("UTF-8") {
        false
    } else {
        return Err(text_err(format!(
            "charset {charset:?} unrecognized (only UTF-8 and US-ASCII supported)"
        )));
    };

    let language_str = language
        .filter(|s| !s.is_empty())
        .ok_or_else(|| text_err("missing language tag in ext-value"))?;
    let language_tag = LanguageTag::new(language_str)
        .map_err(|_| text_err("invalid language tag in ext-value"))?;

    let decoded = percent_decode(&encoded, ascii)
        .map_err(|e| text_err(format!("percent-decode error: {e}")))?;

    Ok(I18nString {
        language: language_tag,
        value: decoded,
    })
}

/// Percent-decode a `value-chars` string (RFC 8187 §3.2).
fn percent_decode(encoded: &str, ascii: bool) -> Result<String, String> {
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut chars = encoded.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().ok_or("incomplete percent-encoding")?;
            let h2 = chars.next().ok_or("incomplete percent-encoding")?;
            let hex: String = [h1, h2].iter().collect();
            let byte = u8::from_str_radix(&hex, 16)
                .map_err(|_| format!("invalid hex in percent-encoding: {hex:?}"))?;
            if ascii && !byte.is_ascii() {
                return Err("invalid US-ASCII after decoding".into());
            }
            bytes.push(byte);
        } else {
            let mut buf = [0u8; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
    String::from_utf8(bytes).map_err(|e| format!("invalid UTF-8 after decoding: {e}"))
}

fn entry_to_link_context(
    entry: Entry,
    base: Option<&BaseIri<String>>,
) -> Result<LinkContext, LinksetError> {
    let target = resolve(base, &entry.target)
        .map_err(|e| text_err(format!("invalid target URI {:?}: {e}", entry.target)))?;

    let mut rels: Vec<RelType> = Vec::new();
    let mut anchor_str: Option<String> = None;
    let mut type_: Option<MediaType> = None;
    let mut title: Option<String> = None;
    let mut title_i18n: Option<I18nString> = None;
    let mut hreflang: Vec<LanguageTag<String>> = Vec::new();
    let mut media: Option<String> = None;
    let mut exts: Vec<(String, Vec<String>)> = Vec::new();
    let mut exts_i18n: Vec<(String, Vec<I18nString>)> = Vec::new();

    for (name, value) in entry.params {
        let name_lower = name.to_ascii_lowercase();
        match name_lower.as_str() {
            "rel" => {
                let s = plain_value(value)?;
                for rel_str in s.split_ascii_whitespace() {
                    let rel = RelType::new(rel_str)
                        .ok_or_else(|| text_err(format!("invalid rel type: {rel_str:?}")))?;
                    rels.push(rel);
                }
            }
            "anchor" => {
                anchor_str = Some(plain_value(value)?);
            }
            "type" => {
                let s = plain_value(value)?;
                type_ = Some(
                    MediaType::new(s.clone())
                        .map_err(|e| text_err(format!("invalid media type {s:?}: {e}")))?,
                );
            }
            "title" => {
                title = Some(plain_value(value)?);
            }
            "title*" => {
                title_i18n = Some(decode_ext_value(value)?);
            }
            "hreflang" => {
                let s = plain_value(value)?;
                hreflang.push(
                    LanguageTag::new(s.clone())
                        .map_err(|_| text_err(format!("invalid hreflang tag: {s:?}")))?,
                );
            }
            "media" => {
                media = Some(plain_value(value)?);
            }
            // Skip other well-known keys (e.g. "rev" which is deprecated in RFC 8288 §3.3)
            n if ext_token::WELL_KNOWN_KEYS.contains(&n)
                || ext_token::WELL_KNOWN_I18N_KEYS.contains(&n) => {}
            // Extension i18n attribute (key ends with '*')
            n if n.ends_with('*') => {
                let i18n = decode_ext_value(value)?;
                if let Some((_, v)) = exts_i18n.iter_mut().find(|(k, _)| k == n) {
                    v.push(i18n);
                } else {
                    exts_i18n.push((name_lower, vec![i18n]));
                }
            }
            // Extension plain attribute
            n => {
                let s = plain_value(value)?;
                if let Some((_, v)) = exts.iter_mut().find(|(k, _)| k == n) {
                    v.push(s);
                } else {
                    exts.push((name_lower, vec![s]));
                }
            }
        }
    }

    let anchor = if let Some(a) = anchor_str {
        resolve(base, &a).map_err(|e| text_err(format!("invalid anchor URI {a:?}: {e}")))?
    } else if let Some(b) = base {
        Uri::new(b.as_str().to_string())
            .map_err(|e| text_err(format!("base URI is not a valid URI: {e}")))?
    } else {
        return Err(text_err(
            "missing 'anchor' parameter and no base URI provided",
        ));
    };

    if rels.is_empty() {
        return Err(text_err("missing 'rel' parameter"));
    }

    let links: Vec<_> = rels
        .into_iter()
        .map(|rel| {
            let mut link = Link::new(target.clone(), rel);
            link.type_ = type_.clone();
            link.title = title.clone();
            link.title_i18n = title_i18n.clone();
            link.hreflang = hreflang.clone();
            link.media = media.clone();
            for (k, v) in &exts {
                link.set_ext(k, v.clone());
            }
            for (k, v) in &exts_i18n {
                link.set_ext_i18n(k, v.clone());
            }
            link
        })
        .collect();

    LinkContext::new_with(anchor, links)
}

fn entries_to_linkset(
    entries: Vec<Entry>,
    base: Option<&BaseIri<String>>,
) -> Result<Linkset, LinksetError> {
    entries
        .into_iter()
        .map(|e| entry_to_link_context(e, base))
        .collect::<Result<Vec<LinkContext>, _>>()?
        .into_iter()
        .collect::<Result<Linkset, _>>()
}

// ── Serialization helpers ────────────────────────────────────────────────────

fn write_link_value(
    w: &mut impl io::Write,
    link: &Link,
    anchor: UriRef<Cow<str>>,
    pretty: bool,
    rel: &UriRelativizer,
) -> io::Result<()> {
    let sep: &[u8] = if pretty { b"\n   ; " } else { b"; " };

    write!(w, "<{}>", rel.relativize(link.target().as_ref()))?;

    w.write_all(sep)?;
    w.write_all(b"rel=")?;
    write_quoted(w, link.rel().as_str())?;

    if !anchor.is_empty() {
        w.write_all(sep)?;
        w.write_all(b"anchor=")?;
        write_quoted(w, anchor.as_str())?;
    }

    if let Some(t) = &link.type_ {
        w.write_all(sep)?;
        w.write_all(b"type=")?;
        write_quoted(w, t.as_str())?;
    }
    for lang in &link.hreflang {
        w.write_all(sep)?;
        write!(w, "hreflang={}", lang.as_str())?;
    }
    if let Some(title) = &link.title {
        w.write_all(sep)?;
        w.write_all(b"title=")?;
        write_quoted(w, title)?;
    }
    if let Some(i18n) = &link.title_i18n {
        w.write_all(sep)?;
        w.write_all(b"title*=")?;
        write_ext_value(w, i18n)?;
    }
    if let Some(media) = &link.media {
        w.write_all(sep)?;
        w.write_all(b"media=")?;
        write_quoted(w, media)?;
    }
    for (key, values) in link.iter_ext() {
        for value in values {
            w.write_all(sep)?;
            write!(w, "{key}=")?;
            write_quoted(w, value)?;
        }
    }
    for (key, values) in link.iter_ext_i18n() {
        for value in values {
            w.write_all(sep)?;
            write!(w, "{key}=")?;
            write_ext_value(w, value)?;
        }
    }
    Ok(())
}

fn write_quoted(w: &mut impl io::Write, s: &str) -> io::Result<()> {
    w.write_all(b"\"")?;
    for b in s.bytes() {
        match b {
            b'"' => w.write_all(b"\\\"")?,
            b'\\' => w.write_all(b"\\\\")?,
            _ => w.write_all(&[b])?,
        }
    }
    w.write_all(b"\"")
}

fn write_ext_value(w: &mut impl io::Write, i18n: &I18nString) -> io::Result<()> {
    write!(w, "UTF-8'{}'", i18n.language.as_str())?;
    for b in i18n.value.bytes() {
        if is_attr_char(b) {
            w.write_all(&[b])?;
        } else {
            write!(w, "%{b:02X}")?;
        }
    }
    Ok(())
}

/// Return true for [RFC 8187 §3.2](https://www.rfc-editor.org/rfc/rfc8187#section-3.2) `attr-char`.
fn is_attr_char(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'!' | b'#' | b'$' | b'&' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
        )
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;

    #[test]
    fn quoted_string_() {
        let input = r#""\"\\\a""#;
        assert_eq!(quoted_string(&mut input.as_bytes()).unwrap(), r#""\a"#,);
    }

    #[test_case("UTF-8'fr'accentu%c3%a9", Some(("accentué", "fr")); "valid UTF-8")]
    #[test_case("US-ASCII'fr'accent", Some(("accent", "fr")); "valid US-ASCII")]
    #[test_case("US-ASCII'fr'%61ccent", Some(("accent", "fr")); "valid US-ASCII with percent-encoded")]
    #[test_case("UTF-8'fr'accentu%c3", None; "truncated UTF-8")]
    #[test_case("UTF-8'fr'accentu%ff", None; "invalid UTF-8")]
    #[test_case("UTF-8'fr'accentu%c3%a", None; "UTF-8 with incomplete percent-encoding")]
    #[test_case("UTF-8'fr'accentu%c3%az", None; "UTF-8 with invalid percent-encoding")]
    #[test_case("US-ASCII'fr'accentu%c3%a9", None; "non-ASCII")]
    fn ext_value_(input: &str, exp: Option<(&str, &str)>) {
        let ext = ext_value(&mut input.as_bytes()).unwrap();
        if let Some((txt, lang)) = exp {
            let got = decode_ext_value(ext).unwrap();
            let exp = I18nString::new(txt.into(), LanguageTag::new_unchecked(lang.into()));
            assert_eq!(got, exp);
        } else {
            assert!(decode_ext_value(ext).is_err());
        }
    }

    #[test]
    fn missing_rel_is_error() {
        let input = r#"<https://example.com/foo>; anchor="https://example.com/""#;
        assert!(matches!(
            Linkset::from_text_str(input, None),
            Err(LinksetError::Text(_))
        ));
    }

    #[test]
    fn missing_anchor_without_base_is_error() {
        let input = r#"<https://example.com/foo>; rel="item""#;
        assert!(matches!(
            Linkset::from_text_str(input, None),
            Err(LinksetError::Text(_))
        ));
    }

    #[test]
    fn missing_anchor_with_base_ok() {
        let base = Some(Uri::new_unchecked("https://example.com/"));
        let input = r#"<https://example.com/foo>; rel="item""#;
        let ls = Linkset::from_text_str(input, base).unwrap();
        assert_eq!(ls[0].anchor(), "https://example.com/");
        assert_eq!(ls[0][0].target(), "https://example.com/foo");
        assert_eq!(ls[0][0].rel(), "item");
    }

    #[test]
    fn multiple_rels_produce_multiple_links() {
        let input = r#"<https://example.com/foo>; rel="item next"; anchor="https://example.com/""#;
        let ls = Linkset::from_text_str(input, None).unwrap();
        assert_eq!(ls[0].len(), 2);
        assert_eq!(ls[0][0].rel(), "item");
        assert_eq!(ls[0][1].rel(), "next");
    }

    #[test]
    fn extension_uri_rel() {
        let input = r#"<https://example.com/foo>; rel="https://example.com/rel/owns"; anchor="https://example.com/""#;
        let ls = Linkset::from_text_str(input, None).unwrap();
        assert_eq!(ls[0][0].rel(), "https://example.com/rel/owns");
    }

    #[test]
    fn hreflang_multi_params() {
        let input = r#"<https://ex.org/foo>; rel="next"; anchor="https://ex.org/"; hreflang=en; hreflang=de"#;
        let ls = Linkset::from_text_str(input, None).unwrap();
        assert_eq!(ls[0][0].hreflang.len(), 2);
        assert_eq!(ls[0][0].hreflang[0].as_str(), "en");
        assert_eq!(ls[0][0].hreflang[1].as_str(), "de");
    }

    #[test]
    fn ext_i18n_title_star() {
        // title*=UTF-8'de'n%c3%a4chstes%20Kapitel  →  "nächstes Kapitel"
        let input = r#"<https://ex.org/foo>; rel="next"; anchor="https://ex.org/"; title*=UTF-8'de'n%c3%a4chstes%20Kapitel"#;
        let ls = Linkset::from_text_str(input, None).unwrap();
        let t = ls[0][0].title_i18n.as_ref().unwrap();
        assert_eq!(t.language.as_str(), "de");
        assert_eq!(t.value, "nächstes Kapitel");
    }

    #[test]
    fn ext_attribute_multi_value() {
        let input = r#"<https://ex.org/foo>; rel="item"; anchor="https://ex.org/"; bar="barone"; bar="bartwo""#;
        let ls = Linkset::from_text_str(input, None).unwrap();
        assert_eq!(ls[0][0].get_ext("bar").unwrap(), &["barone", "bartwo"]);
    }

    #[test]
    fn ext_i18n_attribute() {
        let input = r#"<https://ex.org/foo>; rel="item"; anchor="https://ex.org/"; baz*=US-ASCII'en'bazvalue"#;
        let ls = Linkset::from_text_str(input, None).unwrap();
        let vals = ls[0][0].get_ext_i18n("baz*").unwrap();
        assert_eq!(vals[0].language.as_str(), "en");
        assert_eq!(vals[0].value, "bazvalue");
    }

    #[test]
    fn non_ascii() {
        // 'é' (U+00E9) encodes as 0xC3 0xA9 in UTF-8 — not valid in application/linkset
        let input = r#"<https://ex.org/foo>; rel="item"; anchor="https://ex.org/"; title="erroné""#;
        assert!(Linkset::from_text_str(input, None).is_err());
    }

    #[test]
    fn serialize_with_base() {
        let base = Some(Uri::new_unchecked("https://ex.org/"));
        let input = r#"<foo>; rel="item""#;
        let ls = Linkset::from_text_str(input, base).unwrap();
        let output = ls.to_text_string(false, base);
        assert_eq!(input, output);
    }

    // Figures 12, 14, 17, 18 have no anchor param so they need a base URI.
    #[test_case("figure 1")]
    #[test_case("figure 2")]
    #[test_case("figure 3")]
    #[test_case("figure 4")]
    #[test_case("figure 5")]
    #[test_case("figure 6")]
    #[test_case("figure 8")]
    #[test_case("figure 10")]
    #[test_case("figure 12")]
    #[test_case("figure 14")]
    #[test_case("figure 17")]
    #[test_case("figure 18")]
    fn parse_text(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.org/resource1"));
        let [_, text] = crate::tests::spec_example(example);
        Linkset::from_text_str(text, base).unwrap();
    }

    #[test_case("figure 1")]
    #[test_case("figure 2")]
    #[test_case("figure 3")]
    #[test_case("figure 4")]
    #[test_case("figure 5")]
    #[test_case("figure 6")]
    #[test_case("figure 8")]
    #[test_case("figure 10")]
    #[test_case("figure 12")]
    #[test_case("figure 14")]
    #[test_case("figure 17")]
    #[test_case("figure 18")]
    fn round_trip_via_string(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.org/resource1"));
        let [_, text] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_text_str(text, base).unwrap();
        let s = ls1.to_text_string(false, None);
        let ls2 = Linkset::from_text_str(&s, base).unwrap();
        assert_eq!(ls1, ls2);
    }

    #[test_case("figure 1")]
    #[test_case("figure 2")]
    #[test_case("figure 3")]
    #[test_case("figure 4")]
    #[test_case("figure 5")]
    #[test_case("figure 6")]
    #[test_case("figure 8")]
    #[test_case("figure 10")]
    #[test_case("figure 12")]
    #[test_case("figure 14")]
    #[test_case("figure 17")]
    #[test_case("figure 18")]
    fn round_trip_via_string_pretty(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.org/resource1"));
        let [_, text] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_text_str(text, base).unwrap();
        let s = ls1.to_text_string(true, None);
        let ls2 = Linkset::from_text_str(&s, base).unwrap();
        assert_eq!(ls1, ls2);
    }

    #[test_case("figure 1")]
    #[test_case("figure 2")]
    #[test_case("figure 3")]
    #[test_case("figure 4")]
    #[test_case("figure 5")]
    #[test_case("figure 6")]
    #[test_case("figure 8")]
    #[test_case("figure 10")]
    #[test_case("figure 12")]
    #[test_case("figure 14")]
    #[test_case("figure 17")]
    #[test_case("figure 18")]
    fn round_trip_via_bytes(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.org/resource1"));
        let [_, text] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_text_str(text, base).unwrap();
        let vec = ls1.to_text_vec(false, None);
        let ls2 = Linkset::from_text_slice(&vec, base).unwrap();
        assert_eq!(ls1, ls2);
    }

    #[test_case("figure 1")]
    #[test_case("figure 2")]
    #[test_case("figure 3")]
    #[test_case("figure 4")]
    #[test_case("figure 5")]
    #[test_case("figure 6")]
    #[test_case("figure 8")]
    #[test_case("figure 10")]
    #[test_case("figure 12")]
    #[test_case("figure 14")]
    #[test_case("figure 17")]
    #[test_case("figure 18")]
    fn round_trip_via_bytes_pretty(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.org/resource1"));
        let [_, text] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_text_str(text, base).unwrap();
        let vec = ls1.to_text_vec(true, None);
        let ls2 = Linkset::from_text_slice(&vec, base).unwrap();
        assert_eq!(ls1, ls2);
    }

    #[test_case("figure 1")]
    #[test_case("figure 2")]
    #[test_case("figure 3")]
    #[test_case("figure 4")]
    #[test_case("figure 5")]
    #[test_case("figure 6")]
    #[test_case("figure 8")]
    #[test_case("figure 10")]
    #[test_case("figure 12")]
    #[test_case("figure 14")]
    #[test_case("figure 17")]
    #[test_case("figure 18")]
    fn round_trip_via_io(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.org/resource1"));
        let [_, text] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_text_str(text, base).unwrap();
        let mut buf = Vec::new();
        ls1.to_text_writer(&mut buf, false, None).unwrap();
        let ls2 = Linkset::from_text_slice(&buf, base).unwrap();
        assert_eq!(ls1, ls2);
    }

    #[test_case("figure 1")]
    #[test_case("figure 2")]
    #[test_case("figure 3")]
    #[test_case("figure 4")]
    #[test_case("figure 5")]
    #[test_case("figure 6")]
    #[test_case("figure 8")]
    #[test_case("figure 10")]
    #[test_case("figure 12")]
    #[test_case("figure 14")]
    #[test_case("figure 17")]
    #[test_case("figure 18")]
    fn round_trip_via_io_pretty(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.org/resource1"));
        let [_, text] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_text_str(text, base).unwrap();
        let mut buf = Vec::new();
        ls1.to_text_writer(&mut buf, true, None).unwrap();
        let ls2 = Linkset::from_text_slice(&buf, base).unwrap();
        assert_eq!(ls1, ls2);
    }

    #[test_case("figure 1")]
    #[test_case("figure 2")]
    #[test_case("figure 3")]
    #[test_case("figure 4")]
    #[test_case("figure 5")]
    #[test_case("figure 6")]
    #[test_case("figure 8")]
    #[test_case("figure 10")]
    #[test_case("figure 12")]
    #[test_case("figure 14")]
    #[test_case("figure 17")]
    #[test_case("figure 18")]
    fn round_trip_via_io_with_base(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.org/resource1"));
        let [_, text] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_text_str(text, base).unwrap();
        let mut buf = Vec::new();
        ls1.to_text_writer(&mut buf, false, base).unwrap();
        let ls2 = Linkset::from_text_slice(&buf, base).unwrap();
        assert_eq!(ls1, ls2);
    }
}
