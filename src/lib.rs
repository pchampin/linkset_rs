pub mod error;
pub mod json;
pub mod model;
pub mod text;

pub use model::Linkset;

#[cfg(test)]
pub(crate) mod tests {
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
    "#,
        ],
    ];
}
