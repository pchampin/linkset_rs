//! Rust implementation of the Linkset media types [RFC 9264]
//!
//! [RFC 9264]: https://www.rfc-editor.org/rfc/rfc9264.html
//!
//! # Example: reading and using linkset
//! ```
//! # fn test() -> Result<(), Box<dyn std::error::Error>> {
//! #
//! use linkset::{Linkset, model::Uri};
//! let uri = Uri::new_unchecked("https://example.org/page1");
//!
//! // this might typically come from an HTTP response's Link header
//! let link_header = "<https://example.org/page2>; rel=next, <https://example.org/home>; rel=up";
//!
//! let linkset = Linkset::from_text_str(link_header, Some(uri))?;
//! let back = linkset.find(&uri, "prev").next().or_else(|| linkset.find(&uri, "up").next());
//! assert!(back.is_some());
//! #
//! # Ok(()) } test();
//! ```
//!
//! # Example: producing a linkset JSON representation
//!
//! ```
//! # fn test() -> Result<(), Box<dyn std::error::Error>> {
//! #
//! use linkset::{Linkset, model::{Link, LinkContext, RelType, Uri}};
//!
//! let page1 = Uri::new_unchecked("https://example.org/page1".to_string());
//! let page2 = Uri::new_unchecked("https://example.org/page2".to_string());
//! let home = Uri::new_unchecked("https://example.org/page1".to_string());
//!
//! let linkset: Linkset = LinkContext::new_with(
//!   page1,
//!   vec![
//!     Link::new(page2, RelType::new_reg_unchecked("next")),
//!     Link::new(home, RelType::new_reg_unchecked("up")),
//!   ]
//! )?.into();
//!
//! let json = linkset.to_json_string(true);
//!
//! # assert_eq!(linkset, Linkset::from_json_str(&json, None).unwrap());
//! # Ok(()) } test();
//! ```
//!

pub mod error;
pub mod json;
pub mod model;
pub mod text;

pub use model::Linkset;

// ── Interop tests (step 5) ────────────────────────────────────────────────────

#[cfg(test)]
pub(crate) mod tests {
    use sophia_iri::uri::Uri;
    use test_case::test_case;

    use crate::Linkset;

    /// Get a specific example from SPEC_EXAMPLES by its name.
    ///
    /// Will panic if the name is not found in SPEC_EXAMPLES.
    pub fn spec_example(name: &str) -> [&'static str; 2] {
        SPEC_EXAMPLES
            .iter()
            .find(|a| a[0] == name)
            .map(|[_, j, t]| [*j, *t])
            .unwrap()
    }

    /// All relevant examples from RFC 9264,
    /// converted when needed in both formats.
    /// The values are (example_name, json, text).
    pub static SPEC_EXAMPLES: &[[&str; 3]] = &[
        [
            "figure 1",
            r#"
{ "linkset":
  [
    { "anchor": "https://example.net/bar",
      "next": [
        {"href": "https://example.com/foo"}
      ]
    }
  ]
}
    "#,
            r#"
<https://example.com/foo>
   ; rel="next"
   ; anchor="https://example.net/bar"
    "#,
        ],
        [
            "figure 2",
            r#"
{ "linkset":
  [
    { "anchor": "https://example.net/bar",
      "item": [
        {"href": "https://example.com/foo1"},
        {"href": "https://example.com/foo2"}
      ]
    }
  ]
}
    "#,
            r#"
<https://example.com/foo1>
   ; rel="item"
   ; anchor="https://example.net/bar",
<https://example.com/foo2>
   ; rel="item"
   ; anchor="https://example.net/bar"
    "#,
        ],
        [
            "figure 3",
            r#"
{ "linkset":
  [
    { "anchor": "https://example.net/bar",
      "next": [
        {"href": "https://example.com/foo1"}
      ]
    },
    { "anchor": "https://example.net/boo",
      "https://example.com/relations/baz" : [
        {"href": "https://example.com/foo2"}
      ]
    }
  ]
}
    "#,
            r#"
<https://example.com/foo1>
   ; rel="next"
   ; anchor="https://example.net/bar",
<https://example.com/foo2>
   ; rel="https://example.com/relations/baz"
   ; anchor="https://example.net/boo"
    "#,
        ],
        [
            "figure 4",
            r#"
{ "linkset":
  [
    { "anchor": "https://example.net/bar",
      "next": [
        { "href":     "https://example.com/foo",
          "type":     "text/html",
          "hreflang": [ "en" , "de" ]
        }
      ]
    }
  ]
}
    "#,
            r#"
<https://example.com/foo>
   ; rel="next"
   ; anchor="https://example.net/bar"
   ; type="text/html"
   ; hreflang=en
   ; hreflang=de
    "#,
        ],
        [
            "figure 5",
            r#"
{ "linkset":
  [
    { "anchor": "https://example.net/bar",
      "next": [
        { "href":     "https://example.com/foo",
          "type":     "text/html",
          "hreflang": [ "en" , "de" ],
          "title":    "Next chapter",
          "title*":   [ { "value": "nächstes Kapitel" ,
                          "language" : "de" } ]
        }
      ]
    }
  ]
}
    "#,
            r#"
<https://example.com/foo>
   ; rel="next"
   ; anchor="https://example.net/bar"
   ; type="text/html"
   ; hreflang=en
   ; hreflang=de
   ; title="Next chapter"
   ; title*=UTF-8'de'n%c3%a4chstes%20Kapitel
    "#,
        ],
        [
            "figure 6",
            r#"
{ "linkset":
  [
    { "anchor": "https://example.net/bar",
      "next": [
        { "href": "https://example.com/foo",
          "type": "text/html",
          "foo":  [ "foovalue" ],
          "bar":  [ "barone", "bartwo" ],
          "baz*": [ { "value": "bazvalue" ,
                      "language" : "en" } ]
        }
      ]
    }
  ]
}
    "#,
            r#"
<https://example.com/foo>
   ; rel="next"
   ; anchor="https://example.net/bar"
   ; type="text/html"
   ; foo="foovalue"
   ; bar="barone"
   ; bar="bartwo"
   ; baz*=US-ASCII'en'bazvalue
    "#,
        ],
        [
            "figure 8",
            r#"
{
    "linkset":[
        {
           "anchor": "https://example.org/resource1",
           "author": [
               { "href": "https://authors.example.net/johndoe",
                 "type": "application/rdf+xml"
               }
           ],
           "latest-version": [
               { "href": "https://example.org/resource1?version=3",
                 "type": "text/html"
               }
           ],
           "memento": [
               { "href": "https://example.org/resource1?version=1",
                 "type": "text/html",
                 "datetime": ["Thu, 13 Jun 2019 09:34:33 GMT"]
               },
               { "href": "https://example.org/resource1?version=2",
                 "type": "text/html",
                 "datetime": ["Sun, 21 Jul 2019 12:22:04 GMT"]
               }
           ]
        },
        {
           "anchor": "https://example.org/resource1?version=3",
           "predecessor-version": [
               { "href": "https://example.org/resource1?version=2",
                 "type": "text/html"
               }
           ]
        },
        {
           "anchor": "https://example.org/resource1?version=2",
           "predecessor-version": [
               { "href": "https://example.org/resource1?version=1",
                 "type": "text/html"
               }
           ]
        },
        {
           "anchor": "https://example.org/resource1#comment=1",
           "author": [
               { "href": "https://authors.example.net/alice" }
           ]
        }
    ]
}
    "#,
            r#"
<https://authors.example.net/johndoe>
   ; rel="author"
   ; type="application/rdf+xml"
   ; anchor="https://example.org/resource1",
<https://example.org/resource1?version=3>
   ; rel="latest-version"
   ; type="text/html"
   ; anchor="https://example.org/resource1",
<https://example.org/resource1?version=2>
   ; rel="predecessor-version"
   ; type="text/html"
   ; anchor="https://example.org/resource1?version=3",
<https://example.org/resource1?version=1>
   ; rel="predecessor-version"
   ; type="text/html"
   ; anchor="https://example.org/resource1?version=2",
<https://example.org/resource1?version=1>
   ; rel="memento"
   ; type="text/html"
   ; datetime="Thu, 13 Jun 2019 09:34:33 GMT"
   ; anchor="https://example.org/resource1",
<https://example.org/resource1?version=2>
   ; rel="memento"
   ; type="text/html"
   ; datetime="Sun, 21 Jul 2019 12:22:04 GMT"
   ; anchor="https://example.org/resource1",
<https://authors.example.net/alice>
   ; rel="author"
   ; anchor="https://example.org/resource1#comment=1"
    "#,
        ],
        [
            "figure 10",
            r#"
{ "linkset":
  [
    { "anchor": "https://example.org/resource1",
      "author": [
        { "href": "https://authors.example.net/johndoe",
          "type": "application/rdf+xml"
        }
      ],
      "memento": [
        { "href": "https://example.org/resource1?version=1",
          "type": "text/html",
          "datetime": "Thu, 13 Jun 2019 09:34:33 GMT"
        },
        { "href": "https://example.org/resource1?version=2",
          "type": "text/html",
          "datetime": "Sun, 21 Jul 2019 12:22:04 GMT"
        }
      ],
      "latest-version": [
        { "href": "https://example.org/resource1?version=3",
          "type": "text/html"
        }
      ]
    },
    { "anchor": "https://example.org/resource1?version=3",
      "predecessor-version": [
        { "href": "https://example.org/resource1?version=2",
          "type": "text/html"
        }
      ]
    },
    { "anchor": "https://example.org/resource1?version=2",
      "predecessor-version": [
        { "href": "https://example.org/resource1?version=1",
          "type": "text/html"
        }
      ]
    },
    { "anchor": "https://example.org/resource1#comment=1",
      "author": [
        { "href": "https://authors.example.net/alice"}
      ]
    }
  ]
}
    "#,
            r#"
<https://authors.example.net/johndoe>
   ; rel="author"
   ; type="application/rdf+xml"
   ; anchor="https://example.org/resource1",
<https://example.org/resource1?version=1>
   ; rel="memento"
   ; type="text/html"
   ; datetime="Thu, 13 Jun 2019 09:34:33 GMT"
   ; anchor="https://example.org/resource1",
<https://example.org/resource1?version=2>
   ; rel="memento"
   ; type="text/html"
   ; datetime="Sun, 21 Jul 2019 12:22:04 GMT"
   ; anchor="https://example.org/resource1",
<https://example.org/resource1?version=3>
   ; rel="latest-version"
   ; type="text/html"
   ; anchor="https://example.org/resource1",
<https://example.org/resource1?version=2>
   ; rel="predecessor-version"
   ; type="text/html"
   ; anchor="https://example.org/resource1?version=3",
<https://example.org/resource1?version=1>
   ; rel="predecessor-version"
   ; type="text/html"
   ; anchor="https://example.org/resource1?version=2",
<https://authors.example.net/alice>
   ; rel="author"
   ; anchor="https://example.org/resource1#comment=1"
    "#,
        ],
        [
            "figure 12",
            r#"{
  "linkset": [{
    "linkset": [{
      "href": "https://example.org/links/resource1",
      "type": "application/linkset+json"
    }]
  }]
}
     "#,
            r#"
<https://example.org/links/resource1>
      ; rel="linkset"
      ; type="application/linkset+json"
    "#,
        ],
        [
            "figure 14",
            r#"{
  "linkset": [{
    "linkset": [{
      "href": "https://id.gs1.org/01/9506000134352?linkType=all",
      "type": "application/linkset+json",
      "profile": ["https://www.gs1.org/voc/?show=linktypes"]
    }]
  }]
}
     "#,
            r#"
<https://id.gs1.org/01/9506000134352?linkType=all>
      ; rel="linkset"
      ; type="application/linkset+json"
      ; profile="https://www.gs1.org/voc/?show=linktypes"
        "#,
        ],
        [
            "figure 17",
            r#"{
  "linkset": [{
    "profile": [{
      "href": "https://www.gs1.org/voc/?show=linktypes"
    }]
  }]
}
     "#,
            r#"
<https://www.gs1.org/voc/?show=linktypes>; rel="profile"
        "#,
        ],
        [
            "figure 18",
            r#"
{ "linkset":
  [
    { "anchor": "https://id.gs1.org/01/9506000134352?linkType=all",
      "profile": [
            {"href": "https://www.gs1.org/voc/?show=linktypes"}
      ]
    },
     { "anchor": "https://id.gs1.org/01/9506000134352",
       "https://gs1.org/voc/whatsInTheBox": [
         {"href": "https://example.com/en/packContents/GB"}
       ]
    }
  ]
}
    "#,
            r#"
<https://www.gs1.org/voc/?show=linktypes>
      ; rel="profile"
      ; anchor="https://id.gs1.org/01/9506000134352?linkType=all",
<https://example.com/en/packContents/GB>
      ; rel="https://gs1.org/voc/whatsInTheBox"
      ; anchor="https://id.gs1.org/01/9506000134352"
    "#,
        ],
    ];

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
    fn json_and_text_parse_equal(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.org/resource1"));
        let [json, text] = crate::tests::spec_example(example);
        let ls_json = Linkset::from_json_str(json, base).unwrap();
        let ls_text = Linkset::from_text_str(text, base).unwrap();
        assert_eq!(ls_json, ls_text);
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
    fn json_to_text_and_back(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.org/resource1"));
        let [json, _] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_json_str(json, base).unwrap();
        let text = ls1.to_text_string(false);
        let ls2 = Linkset::from_text_str(&text, base).unwrap();
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
    fn text_to_json_and_back(example: &str) {
        let base = Some(Uri::new_unchecked("https://example.org/resource1"));
        let [_, text] = crate::tests::spec_example(example);
        let ls1 = Linkset::from_text_str(text, base).unwrap();
        let json_str = ls1.to_json_string(false);
        let ls2 = Linkset::from_json_str(&json_str, base).unwrap();
        assert_eq!(ls1, ls2);
    }
}
