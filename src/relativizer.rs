use std::borrow::Cow;

use sophia_iri::{
    relativize::Relativizer,
    uri::{Uri, UriRef},
};

pub struct UriRelativizer<'a>(Option<Relativizer<&'a str>>);

impl<'a> UriRelativizer<'a> {
    pub fn new(base: Option<Uri<&'a str>>) -> Self {
        Self(base.map(|uri| Relativizer::new(uri.into_iri().to_base(), 1)))
    }

    pub fn relativize<'b>(&self, uri: Uri<&'b str>) -> UriRef<Cow<'b, str>> {
        if let Some(rel) = &self.0
            && let Some(iri_ref) = rel.relativize(uri.into_iri())
        {
            UriRef::new_unchecked(iri_ref.unwrap())
        } else {
            uri.into_uri_ref().map_unchecked(Cow::from)
        }
    }
}
