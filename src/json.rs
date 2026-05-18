//! I provide implementation for parsng and serializing [`Linkset`]s
//! in the JSON format defined by [RFC 9264], `application/linkset+json`.
//!
//! See:
//! * [`Linkset::from_json_reader`]
//! * [`Linkset::from_json_slice`]
//! * [`Linkset::from_json_str`]
//! * [`Linkset::from_json_value`]
//! * [`Linkset::to_json_string`]
//! * [`Linkset::to_json_value`]
//! * [`Linkset::to_json_vec`]
//! * [`Linkset::to_json_writer`]
//!
//! [RFC 9264]: https://www.rfc-editor.org/rfc/rfc9264.html

use std::borrow::Borrow;
use std::io;

use serde_json::Value;
use sophia_bcp47::LanguageTag;
use sophia_iri::{
    resolve::BaseIri,
    uri::{InvalidUri, Uri, UriRef},
};

use crate::{
    Linkset,
    error::LinksetError,
    model::{I18nString, Link, LinkContext, MediaType, RelType, ext_token},
};

impl Linkset {
    /// Create a [`Linkset`] from a JSON value complying with `application/linkset+json`
    /// as defined by [RFC 9264 Sec. 4.2](https://www.rfc-editor.org/rfc/rfc9264.html#name-json-document-format-applic).
    pub fn from_json_value(
        value: impl Borrow<Value>,
        base: Option<Uri<&str>>,
    ) -> Result<Linkset, LinksetError> {
        let value = value.borrow();

        let obj = value
            .as_object()
            .ok_or_else(|| json_err("top-level value must be an object"))?;

        let linkset_arr = obj
            .get("linkset")
            .ok_or_else(|| json_err("missing \"linkset\" key"))?
            .as_array()
            .ok_or_else(|| json_err("\"linkset\" must be an array"))?;

        let base = base.map(|b| b.map_unchecked(String::from).into_iri().to_base());

        let contexts = linkset_arr
            .iter()
            .enumerate()
            .map(|(i, v)| parse_context(v, i, base.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;

        contexts.into_iter().collect::<Result<Linkset, _>>()
    }

    /// Create a [`Linkset`] from a JSON text complying with `application/linkset+json`
    /// as defined by [RFC 9264 Sec. 4.2](https://www.rfc-editor.org/rfc/rfc9264.html#name-json-document-format-applic).
    pub fn from_json_str(str: &str, base: Option<Uri<&str>>) -> Result<Linkset, LinksetError> {
        Linkset::from_json_value(serde_json::from_str::<Value>(str)?, base)
    }

    /// Create a [`Linkset`] from a JSON bytes complying with `application/linkset+json`
    /// as defined by [RFC 9264 Sec. 4.2](https://www.rfc-editor.org/rfc/rfc9264.html#name-json-document-format-applic).
    pub fn from_json_slice(slice: &[u8], base: Option<Uri<&str>>) -> Result<Linkset, LinksetError> {
        Linkset::from_json_value(serde_json::from_slice::<Value>(slice)?, base)
    }

    /// Create a [`Linkset`] from a JSON stream complying with `application/linkset+json`
    /// as defined by [RFC 9264 Sec. 4.2](https://www.rfc-editor.org/rfc/rfc9264.html#name-json-document-format-applic).
    pub fn from_json_reader(
        reader: impl io::Read,
        base: Option<Uri<&str>>,
    ) -> Result<Linkset, LinksetError> {
        Linkset::from_json_value(serde_json::from_reader::<_, Value>(reader)?, base)
    }

    /// Create a JSON value from [`Linkset`], complying with `application/linkset+json`
    /// as defined by [RFC 9264 Sec. 4.2](https://www.rfc-editor.org/rfc/rfc9264.html#name-json-document-format-applic).
    pub fn to_json_value(&self) -> Value {
        let contexts: Vec<Value> = self.iter().map(context_to_value).collect();
        let mut map = serde_json::Map::new();
        map.insert("linkset".into(), Value::Array(contexts));
        Value::Object(map)
    }

    /// Create a JSON string from [`Linkset`], complying with `application/linkset+json`
    /// as defined by [RFC 9264 Sec. 4.2](https://www.rfc-editor.org/rfc/rfc9264.html#name-json-document-format-applic).
    pub fn to_json_string(&self, pretty: bool) -> String {
        if pretty {
            format!("{}", self.to_json_value())
        } else {
            format!("{:#}", self.to_json_value())
        }
    }

    /// Create JSON bytes from [`Linkset`], complying with `application/linkset+json`
    /// as defined by [RFC 9264 Sec. 4.2](https://www.rfc-editor.org/rfc/rfc9264.html#name-json-document-format-applic).
    pub fn to_json_vec(&self, pretty: bool) -> Vec<u8> {
        if pretty {
            serde_json::to_vec_pretty(&self.to_json_value())
        } else {
            serde_json::to_vec(&self.to_json_value())
        }
        .unwrap()
        // we are serializing a serde_json::Value to JSON,
        // there is no other reason for the serialization to fail
    }

    /// Write [`Linkset`] to a stream as JSON, complying with `application/linkset+json`
    /// as defined by [RFC 9264 Sec. 4.2](https://www.rfc-editor.org/rfc/rfc9264.html#name-json-document-format-applic).
    pub fn to_json_writer(&self, writer: impl io::Write, pretty: bool) -> io::Result<()> {
        if pretty {
            serde_json::to_writer_pretty(writer, &self.to_json_value())
        } else {
            serde_json::to_writer(writer, &self.to_json_value())
        }
        .map_err(|err| {
            if let Some(io_err_kind) = err.io_error_kind() {
                io_err_kind.into()
            } else {
                unreachable!()
                // we are serializing a serde_json::Value to JSON,
                // there is no other reason for the serialization to fail
            }
        })
    }
}

// ---------------- Parsing ----------------

fn json_err(msg: impl Into<String>) -> LinksetError {
    LinksetError::Json(msg.into())
}

fn parse_context(
    value: &Value,
    idx: usize,
    base: Option<&BaseIri<String>>,
) -> Result<LinkContext, LinksetError> {
    let obj = value
        .as_object()
        .ok_or_else(|| json_err(format!("linkset[{idx}]: must be an object")))?;

    let anchor_str = match obj.get("anchor") {
        None => {
            if base.is_none() {
                Err(json_err(format!("linkset[{idx}]: missing \"anchor\"")))?
            } else {
                ""
            }
        }
        Some(val) => val
            .as_str()
            .ok_or_else(|| json_err(format!("linkset[{idx}].anchor: must be a string")))?,
    };

    let anchor = resolve(base, anchor_str).map_err(|e| {
        json_err(format!(
            "linkset[{idx}].anchor: invalid URI {anchor_str:?}: {e}"
        ))
    })?;

    let mut links = Vec::new();

    for (key, val) in obj {
        if key == "anchor" {
            continue;
        }
        let rel = RelType::new(key.clone())
            .ok_or_else(|| json_err(format!("linkset[{idx}]: invalid relation type {key:?}")))?;
        let targets = val
            .as_array()
            .ok_or_else(|| json_err(format!("linkset[{idx}][{key:?}]: must be an array")))?;
        for (j, target) in targets.iter().enumerate() {
            links.push(parse_link_target(target, idx, key, j, rel.clone(), base)?);
        }
    }

    LinkContext::new_with(anchor, links)
}

/// Precondition: base must be known to be a URI, not just any IRI.
fn resolve(base: Option<&BaseIri<String>>, txt: &str) -> Result<Uri<String>, InvalidUri> {
    if let Some(base) = base {
        let uri_ref = UriRef::new(txt.to_string())?.into_iri_ref();
        Ok(base.resolve(uri_ref).to_uri_unchecked())
    } else {
        Uri::new(txt.to_string())
    }
}

fn parse_link_target(
    value: &Value,
    ctx_idx: usize,
    rel_key: &str,
    target_idx: usize,
    rel: RelType,
    base: Option<&BaseIri<String>>,
) -> Result<Link, LinksetError> {
    let path = || format!("linkset[{ctx_idx}][{rel_key:?}][{target_idx}]");

    let obj = value
        .as_object()
        .ok_or_else(|| json_err(format!("{}: must be an object", path())))?;

    let href_str = obj
        .get("href")
        .ok_or_else(|| json_err(format!("{}: missing \"href\"", path())))?
        .as_str()
        .ok_or_else(|| json_err(format!("{}.href: must be a string", path())))?;

    let target = resolve(base, href_str)
        .map_err(|e| json_err(format!("{}.href: invalid URI {href_str:?}: {e}", path())))?;

    let mut link = Link::new(target, rel);

    if let Some(v) = obj.get("type") {
        let s = v
            .as_str()
            .ok_or_else(|| json_err(format!("{}.type: must be a string", path())))?;
        link.type_ = Some(
            MediaType::new(s.to_string())
                .map_err(|e| json_err(format!("{}.type: invalid media-type {e}", path())))?,
        );
    }

    if let Some(v) = obj.get("title") {
        link.title = Some(
            v.as_str()
                .ok_or_else(|| json_err(format!("{}.title: must be a string", path())))?
                .to_string(),
        );
    }

    if let Some(v) = obj.get("title*") {
        let arr = v
            .as_array()
            .ok_or_else(|| json_err(format!("{}[\"title*\"]: must be an array", path())))?;
        if arr.len() != 1 {
            Err(json_err(format!(
                "{}[\"title*\"]: must contain exactly one value",
                path()
            )))?;
        }
        if let Some(first) = arr.first() {
            link.title_i18n = Some(
                parse_i18n_value(first)
                    .map_err(|msg| json_err(format!("{}[\"title*\"][0]{msg}", path())))?,
            );
        }
    }

    if let Some(v) = obj.get("hreflang") {
        let arr = v
            .as_array()
            .ok_or_else(|| json_err(format!("{}.hreflang: must be an array", path())))?;
        link.hreflang = arr
            .iter()
            .enumerate()
            .map(|(k, v)| {
                v.as_str()
                    .ok_or_else(|| json_err(format!("{}.hreflang[{k}]: must be a string", path())))
                    .and_then(|txt| {
                        LanguageTag::new(txt.to_string()).map_err(|_| {
                            json_err(format!("{}.hreflang[{k}]: must be a string", path()))
                        })
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
    }

    if let Some(v) = obj.get("media") {
        link.media = Some(
            v.as_str()
                .ok_or_else(|| json_err(format!("{}.media: must be a string", path())))?
                .to_string(),
        );
    }

    for (key, val) in obj {
        if !ext_token::is_valid_token(key) {
            return Err(json_err(format!(
                "{}: invalid extension attribute key {key:?}",
                path(),
            )));
        }
        if key == "href"
            || ext_token::WELL_KNOWN_KEYS.contains(&key.as_str())
            || ext_token::WELL_KNOWN_I18N_KEYS.contains(&key.as_str())
        {
            continue;
        }
        let values: Box<dyn Iterator<Item = &Value>> = match val {
            Value::Array(values) => Box::new(values.iter()),
            // The spec requires an array, but single values are also accepted
            // (if only because Example 10 in the spec uses a single string rather than an array...)
            Value::Object(_) | Value::String(_) => Box::new(std::iter::once(val)),
            _ => {
                return Err(json_err(format!(
                    "{}[{key:?}] (extension attribute): must be an array",
                    path()
                )));
            }
        };
        // .as_array()
        // .ok_or_else(|| json_err(format!("{}[{key:?}]: must be an array", path())))?;
        if key.ends_with('*') {
            let values: Vec<I18nString> = values
                .enumerate()
                .map(|(k, v)| {
                    parse_i18n_value(v)
                        .map_err(|msg| json_err(format!("{}[{key:?}][{k}]{msg}", path())))
                })
                .collect::<Result<_, _>>()?;
            if !values.is_empty() {
                link.set_ext_i18n(key, values);
            }
        } else {
            let values: Vec<String> = values
                .enumerate()
                .map(|(k, v)| {
                    v.as_str()
                        .ok_or_else(|| {
                            json_err(format!("{}[{key:?}][{k}]: must be a string", path()))
                        })
                        .map(str::to_string)
                })
                .collect::<Result<_, _>>()?;
            if !values.is_empty() {
                link.set_ext(key, values);
            }
        }
    }
    Ok(link)
}

fn parse_i18n_value(value: &Value) -> Result<I18nString, &'static str> {
    let obj = value.as_object().ok_or(": i18n value must be an object")?;

    let value_str = obj
        .get("value")
        .ok_or(": i18n value missing \"value\" field")?
        .as_str()
        .ok_or(".value: must be a string")?;

    let language_str = obj
        .get("language")
        .ok_or(": i18n value missing \"language\" field")?
        .as_str()
        .ok_or(".language: must be a string")?;

    let language = LanguageTag::new(language_str.to_string())
        .map_err(|_| ".language: must be a valid BCP47 language tag")?;

    Ok(I18nString {
        language,
        value: value_str.to_string(),
    })
}

// ---------------- Serialization ----------------

fn context_to_value(ctx: &LinkContext) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "anchor".into(),
        Value::String(ctx.anchor().as_str().to_string()),
    );

    for link in ctx {
        let rel = link.rel().as_str().to_string();
        let target_val = link_to_value(link);
        map.entry(rel)
            .or_insert_with(|| Value::Array(vec![]))
            .as_array_mut()
            .unwrap()
            .push(target_val);
    }

    Value::Object(map)
}

fn link_to_value(link: &Link) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "href".into(),
        Value::String(link.target().as_str().to_string()),
    );

    if let Some(type_) = &link.type_ {
        map.insert("type".into(), Value::String(type_.as_str().to_string()));
    }
    if let Some(title) = &link.title {
        map.insert("title".into(), Value::String(title.clone()));
    }
    if let Some(title_i18n) = &link.title_i18n {
        map.insert(
            "title*".into(),
            Value::Array(vec![i18n_to_value(title_i18n)]),
        );
    }
    if !link.hreflang.is_empty() {
        map.insert(
            "hreflang".into(),
            Value::Array(
                link.hreflang
                    .iter()
                    .map(|tag| Value::String(tag.as_str().to_string()))
                    .collect(),
            ),
        );
    }
    if let Some(media) = &link.media {
        map.insert("media".into(), Value::String(media.clone()));
    }

    for (key, values) in link.iter_ext() {
        map.insert(
            key.to_string(),
            Value::Array(values.iter().map(|s| Value::String(s.clone())).collect()),
        );
    }
    for (key, values) in link.iter_ext_i18n() {
        map.insert(
            key.to_string(),
            Value::Array(values.iter().map(i18n_to_value).collect()),
        );
    }

    Value::Object(map)
}

fn i18n_to_value(v: &I18nString) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "language".into(),
        Value::String(v.language.as_str().to_string()),
    );
    map.insert("value".into(), Value::String(v.value.clone()));
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use serde_json::json;
    use test_case::test_case;

    use super::*;

    #[test]
    fn empty_linkset_array_is_error() {
        let val = json!({"linkset": []});
        assert_eq!(
            Linkset::from_json_value(val, None),
            Err(LinksetError::Empty)
        );
    }

    #[test]
    fn missing_anchor_is_error() {
        let val = json!({"linkset": [{"item": [{"href": "https://example.com/"}]}]});
        assert!(matches!(
            Linkset::from_json_value(val, None),
            Err(LinksetError::Json(_))
        ));
    }

    #[test]
    fn missing_href_is_error() {
        let val = json!({"linkset": [{"anchor": "https://example.com/", "item": [{}]}]});
        assert!(matches!(
            Linkset::from_json_value(val, None),
            Err(LinksetError::Json(_))
        ));
    }

    #[test]
    fn extension_attributes_round_trip() {
        let val = json!({
            "linkset": [{
                "anchor": "https://example.com/",
                "item": [{
                    "href": "https://example.com/items/1",
                    "foo": ["bar", "baz"],
                    "label*": [{"language": "fr", "value": "Bonjour"}]
                }]
            }]
        });
        let ls = Linkset::from_json_value(val, None).unwrap();
        let link = &ls[0][0];
        assert_eq!(link.get_ext("foo").unwrap(), &["bar", "baz"]);
        assert_eq!(link.get_ext_i18n("label*").unwrap()[0].value, "Bonjour");
        let ls2 = Linkset::from_json_value(ls.to_json_value(), None).unwrap();
        assert_eq!(ls, ls2);
    }

    #[test]
    fn multiple_links_same_rel_grouped() {
        let val = json!({
            "linkset": [{
                "anchor": "https://example.com/",
                "item": [
                    {"href": "https://example.com/items/1"},
                    {"href": "https://example.com/items/2"}
                ]
            }]
        });
        let ls = Linkset::from_json_value(val.clone(), None).unwrap();
        assert_eq!(ls[0].len(), 2);
        let serialized = ls.to_json_value();
        // Both items should be in the same array
        let items = serialized["linkset"][0]["item"].as_array().unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn extension_rel_type() {
        let val = json!({
            "linkset": [{
                "anchor": "https://example.com/",
                "https://example.com/rel/owns": [{"href": "https://example.com/items/1"}]
            }]
        });
        let ls = Linkset::from_json_value(val, None).unwrap();
        assert_eq!(ls[0][0].rel(), "https://example.com/rel/owns");
    }

    #[test_case(r#"
{
    "linkset": [
        {
            "anchor": "https://example.com/",
            "item": [
                { "href": "https://example.com/items/1" },
                { "href": "https://example.com/items/2" }
            ]
        },
        {
            "anchor": "https://example.com/items/1",
            "next": [
                { "href": "https://example.com/items/2" }
            ]
        }
    ]
}
    "#, r#"
{
    "linkset": [
        {
            "anchor": "https://example.com/items/1",
            "next": [
                { "href": "https://example.com/items/2" }
            ]
        },
        {
            "anchor": "https://example.com/",
            "item": [
                { "href": "https://example.com/items/2" },
                { "href": "https://example.com/items/1" }
            ]
        }
    ]
}
    "#, true; "is order-independent"
    )]
    #[test_case(r#"
{
    "linkset": [
        {
            "anchor": "https://example.com/",
            "item": [
                { "href": "https://example.com/items/1" },
                { "href": "https://example.com/items/2" }
            ]
        }
    ]
}
    "#, r#"
{
    "linkset": [
        {
            "anchor": "https://example.com/other",
            "item": [
                { "href": "https://example.com/items/1" },
                { "href": "https://example.com/items/2" }
            ]
        }
    ]
}
    "#, false; "is anchor-dependant"
    )]
    #[test_case(r#"
{
    "linkset": [
        {
            "anchor": "https://example.com/",
            "item": [
                { "href": "https://example.com/items/1" }
            ]
        }
    ]
}
    "#, r#"
{
    "linkset": [
        {
            "anchor": "https://example.com/",
            "next": [
                { "href": "https://example.com/items/1", "title": "Item #1" }
            ]
        }
    ]
}
    "#, false; "is rel-dependant"
    )]
    #[test_case(r#"
{
    "linkset": [
        {
            "anchor": "https://example.com/",
            "item": [
                { "href": "https://example.com/items/1" }
            ]
        }
    ]
}
    "#, r#"
{
    "linkset": [
        {
            "anchor": "https://example.com/",
            "item": [
                { "href": "https://example.com/items/2" }
            ]
        }
    ]
}
    "#, false; "is target-dependant"
    )]
    #[test_case(r#"
{
    "linkset": [
        {
            "anchor": "https://example.com/",
            "item": [
                { "href": "https://example.com/items/1" }
            ]
        }
    ]
}
    "#, r#"
{
    "linkset": [
        {
            "anchor": "https://example.com/",
            "item": [
                { "href": "https://example.com/items/1", "title": "Item #1" }
            ]
        }
    ]
}
    "#, false; "is attribute-dependant"
    )]
    fn eq(json1: &str, json2: &str, eq: bool) {
        let json1: Value = serde_json::from_str(json1).unwrap();
        let json2: Value = serde_json::from_str(json2).unwrap();
        let ls1 = Linkset::from_json_value(json1, None).unwrap();
        let ls2 = Linkset::from_json_value(json2, None).unwrap();
        if eq {
            assert_eq!(ls1, ls2);
        } else {
            assert!(dbg!(ls1) != dbg!(ls2));
        }
    }

    #[test]
    fn relative_uri() {
        let json_abs: Value = serde_json::from_str(
            r#"{
            "linkset": [
                {
                    "anchor": "https://example.com/",
                    "item": [
                        { "href": "https://example.com/items/1" },
                        { "href": "https://example.com/items/2" }
                    ]
                },
                {
                    "anchor": "https://example.com/items/1",
                    "next": [
                        { "href": "https://example.com/items/2" }
                    ]
                }
            ]
        }"#,
        )
        .unwrap();
        let json_rel: Value = serde_json::from_str(
            r#"{
            "linkset": [
                {
                    "item": [
                        { "href": "items/1" },
                        { "href": "items/2" }
                    ]
                },
                {
                    "anchor": "items/1",
                    "next": [
                        { "href": "items/2" }
                    ]
                }
            ]
        }"#,
        )
        .unwrap();
        let base = Some(Uri::new_unchecked("https://example.com/"));
        let base_other = Some(Uri::new_unchecked("https://example.org/"));

        let ls_abs = Linkset::from_json_value(json_abs, base_other);
        let ls_rel = Linkset::from_json_value(json_rel, base);
        assert_eq!(ls_abs, ls_rel);
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
    fn round_trip_via_value(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.com/"));
        let [json, _] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_json_str(json, base).unwrap();
        let value = ls1.to_json_value();
        let ls2 = Linkset::from_json_value(value, base).unwrap();
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
    fn round_trip_via_string(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.com/"));
        let [json, _] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_json_str(json, base).unwrap();
        let string = ls1.to_json_string(false);
        let ls2 = Linkset::from_json_str(&string, base).unwrap();
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
        let base = Some(Uri::new_unchecked("https://example.com/"));
        let [json, _] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_json_str(json, base).unwrap();
        let string = ls1.to_json_string(true);
        let ls2 = Linkset::from_json_str(&string, base).unwrap();
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
        let base = Some(Uri::new_unchecked("https://example.com/"));
        let [json, _] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_json_str(json, base).unwrap();
        let vec = ls1.to_json_vec(false);
        let ls2 = Linkset::from_json_slice(&vec, base).unwrap();
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
        let base = Some(Uri::new_unchecked("https://example.com/"));
        let [json, _] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_json_str(json, base).unwrap();
        let vec = ls1.to_json_vec(true);
        let ls2 = Linkset::from_json_slice(&vec, base).unwrap();
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
        let base = Some(Uri::new_unchecked("https://example.com/"));
        let [json, _] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_json_str(json, base).unwrap();
        let mut buffer = Cursor::new(vec![0_u8; 0]);
        ls1.to_json_writer(&mut buffer, false).unwrap();
        let ls2 = Linkset::from_json_reader(&buffer.get_ref()[..], base).unwrap();
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
        let base = Some(Uri::new_unchecked("https://example.com/"));
        let [json, _] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_json_str(json, base).unwrap();
        let mut buffer = Cursor::new(vec![0_u8; 0]);
        ls1.to_json_writer(&mut buffer, true).unwrap();
        let ls2 = Linkset::from_json_reader(&buffer.get_ref()[..], base).unwrap();
        assert_eq!(ls1, ls2);
    }
}
