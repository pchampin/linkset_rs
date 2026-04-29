pub(crate) mod ext_token;
mod link;
mod link_context;
mod media_type;
mod rel_type;

pub use link::{I18nString, Link};
pub use link_context::LinkContext;
pub use media_type::MediaType;
pub use rel_type::RelType;
use sophia_iri::uri::Uri;

use crate::error::LinksetError;

/// A set of typed links organised by context, as defined in
/// [RFC 9264](https://www.rfc-editor.org/rfc/rfc9264).
#[derive(Debug, Clone)]
pub struct Linkset(Vec<LinkContext>);

impl Linkset {
    /// Create a new `Linkset` from a list of link contexts.
    pub fn new(anchor: Uri<String>) -> Self {
        let context = LinkContext::new(anchor);
        Linkset(vec![context])
    }

    /// Get the given context from this link set
    pub fn context(&self, anchor: &str) -> Option<&LinkContext> {
        self.0.iter().find(|c| c.anchor().as_str() == anchor)
    }

    /// Get mutably the given context from this link set
    pub fn context_mut(&mut self, anchor: &str) -> Option<&mut LinkContext> {
        self.0.iter_mut().find(|c| c.anchor().as_str() == anchor)
    }

    /// Add context to this linkset with a given anchor,
    /// reusing an existing one if it exists.
    pub fn add_context(&mut self, anchor: Uri<String>) -> &mut LinkContext {
        let i = {
            if let Some(i) = self.0.iter().position(|c| c.anchor() == &anchor) {
                i
            } else {
                self.0.push(LinkContext::new(anchor));
                self.0.len() - 1
            }
        };
        &mut self.0[i]
    }

    /// Add context to this linkset
    pub fn remove_context(
        &mut self,
        anchor: Uri<String>,
    ) -> Result<Option<LinkContext>, LinksetError> {
        match self
            .0
            .iter()
            .enumerate()
            .find(|(_, c)| c.anchor() == &anchor)
        {
            None => Ok(None),
            Some(_) if self.0.len() == 1 => Err(LinksetError::Empty),
            Some((i, _)) => {
                let last = self.0.len() - 1;
                self.0.swap(i, last);
                Ok(self.0.pop())
            }
        }
    }
}

impl PartialEq for Linkset {
    fn eq(&self, other: &Self) -> bool {
        if self.0.len() != other.0.len() {
            return false;
        }
        for c in &self.0 {
            if Some(c) != other.context(c.anchor()) {
                return false;
            }
        }
        true
    }
}

impl std::ops::Deref for Linkset {
    type Target = Vec<LinkContext>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Index<usize> for Linkset {
    type Output = LinkContext;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Linkset {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl IntoIterator for Linkset {
    type Item = LinkContext;

    type IntoIter = std::vec::IntoIter<LinkContext>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Linkset {
    type Item = &'a LinkContext;

    type IntoIter = std::slice::Iter<'a, LinkContext>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<LinkContext> for Linkset {
    fn from(value: LinkContext) -> Self {
        Linkset(vec![value])
    }
}

impl FromIterator<LinkContext> for Result<Linkset, LinksetError> {
    fn from_iter<T: IntoIterator<Item = LinkContext>>(iter: T) -> Self {
        let mut ret = Linkset(vec![]);
        for c in iter.into_iter() {
            if let Some(prev) = ret.context_mut(c.anchor()) {
                prev.merge(c)?;
            } else {
                ret.0.push(c)
            }
        }
        if ret.0.is_empty() {
            Err(LinksetError::Empty)
        } else {
            Ok(ret)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static ROOT: &str = "https://example.com/";
    static ITEM1: &str = "https://example.com/items/1";
    static ITEM2: &str = "https://example.com/items/2";
    static ITEM3: &str = "https://example.com/items/3";

    #[test]
    fn construct_and_mutate() {
        let anchor = Uri::new_unchecked(ROOT.to_string());
        let mut linkset = Linkset::new(anchor);

        linkset[0]
            .add_link(Link::new(
                Uri::new_unchecked(ITEM1.to_string()),
                RelType::new_reg_unchecked("item"),
            ))
            .unwrap();
        linkset[0]
            .add_link(Link::new(
                Uri::new_unchecked(ITEM2.into()),
                RelType::new_reg_unchecked("item"),
            ))
            .unwrap();

        let item2 = linkset[0][1].target().clone();
        let context = linkset.add_context(linkset[0][0].target().clone());
        context
            .add_link(Link::new(item2, RelType::new_reg_unchecked("next")))
            .unwrap();

        assert_eq!(linkset.len(), 2);
        assert_eq!(linkset[0].anchor(), ROOT);
        assert_eq!(linkset[0].len(), 2);
        assert_eq!(linkset[0][0].target(), ITEM1);
        assert_eq!(linkset[0][0].rel(), "item");
        assert_eq!(linkset[0][1].target(), ITEM2);
        assert_eq!(linkset[0][1].rel(), "item");
        assert_eq!(linkset[1].anchor(), ITEM1);
        assert_eq!(linkset[1].len(), 1);
        assert_eq!(linkset[1][0].target(), ITEM2);
        assert_eq!(linkset[1][0].rel(), "next");

        let anchor = linkset[0].anchor().clone();
        linkset.add_context(anchor);
        assert_eq!(linkset.len(), 2); // existing context was reused

        linkset[0].del_link(0);
        assert_eq!(linkset[0].len(), 1);
        assert_eq!(linkset[0][0].target(), ITEM2);
        assert_eq!(linkset[0][0].rel(), "item");

        let mut similar_link = linkset[0][0].clone();
        similar_link.title = Some("This is item #2".into());
        assert!(linkset[0].add_link(similar_link).is_err());
    }

    #[test]
    fn construct_bottom_up() {
        let anchor = Uri::new_unchecked(ROOT.to_string());
        let link1 = Link::new(
            Uri::new_unchecked(ITEM1.to_string()),
            RelType::new_reg_unchecked("item"),
        );
        let link2 = Link::new(
            Uri::new_unchecked(ITEM2.to_string()),
            RelType::new_reg_unchecked("item"),
        );
        let context1 = LinkContext::new_with(anchor, vec![link1, link2]).unwrap();

        let anchor = context1[0].target().clone();
        let link1 = Link::new(
            context1[1].target().clone(),
            RelType::new_reg_unchecked("next"),
        );
        let context2 = LinkContext::new_with(anchor, vec![link1]).unwrap();

        let linkset = vec![context1, context2]
            .into_iter()
            .collect::<Result<Linkset, _>>()
            .unwrap();

        assert_eq!(linkset.len(), 2);
        assert_eq!(linkset[0].anchor(), ROOT);
        assert_eq!(linkset[0].len(), 2);
        assert_eq!(linkset[0][0].target(), ITEM1);
        assert_eq!(linkset[0][0].rel(), "item");
        assert_eq!(linkset[0][1].target(), ITEM2);
        assert_eq!(linkset[0][1].rel(), "item");
        assert_eq!(linkset[1].anchor(), ITEM1);
        assert_eq!(linkset[1].len(), 1);
        assert_eq!(linkset[1][0].target(), ITEM2);
        assert_eq!(linkset[1][0].rel(), "next");
    }

    #[test]
    fn construct_merging_similar_contexts() {
        let anchor = Uri::new_unchecked(ROOT.to_string());
        // build 3 contexts with the same anchor and one link each,
        // and collect them all into a Linkset.
        let linkset = [ITEM1, ITEM2, ITEM3]
            .iter()
            .map(|uri| {
                Link::new(
                    Uri::new_unchecked(uri.to_string()),
                    RelType::new_reg_unchecked("item"),
                )
            })
            .map(|link| LinkContext::new_with(anchor.clone(), vec![link]).unwrap())
            .collect::<Result<Linkset, _>>()
            .unwrap();

        assert_eq!(linkset.len(), 1);
        assert_eq!(linkset[0].anchor(), ROOT);
        assert_eq!(linkset[0].len(), 3);
        assert_eq!(linkset[0][0].target(), ITEM1);
        assert_eq!(linkset[0][0].rel(), "item");
        assert_eq!(linkset[0][1].target(), ITEM2);
        assert_eq!(linkset[0][1].rel(), "item");
        assert_eq!(linkset[0][2].target(), ITEM3);
        assert_eq!(linkset[0][2].rel(), "item");
    }
}
