use sophia_iri::uri::Uri;

use crate::error::LinksetError;

use super::link::Link;

/// A context URI together with the links it anchors.
///
/// Corresponds to one entry in the `"linkset"` array of the JSON format
/// ([RFC 9264 §4.2](https://www.rfc-editor.org/rfc/rfc9264#section-4.2)).
#[derive(Debug, Clone)]
pub struct LinkContext {
    /// The context URI ([RFC 9264 §3](https://www.rfc-editor.org/rfc/rfc9264#section-3) `anchor`).
    anchor: Uri<String>,
    /// The links anchored at this context.
    links: Vec<Link>,
}

impl LinkContext {
    /// Create a new empty `LinkContext`.
    pub fn new(anchor: Uri<String>) -> Self {
        Self::new_with(anchor, std::iter::empty()).unwrap()
    }

    /// Create a new `LinkContext` with the provided links.
    pub fn new_with(
        anchor: Uri<String>,
        links: impl IntoIterator<Item = Link>,
    ) -> Result<Self, LinksetError> {
        let links: Vec<_> = links.into_iter().collect();
        // check that no two links are the "same" link
        for (i, l1) in links.iter().enumerate() {
            for l2 in &links[i + 1..] {
                if l1.target() == l2.target() && l1.rel() == l2.rel() {
                    return Err(LinksetError::DuplicateLink(
                        anchor,
                        l2.rel().clone(),
                        l2.target().clone(),
                    ));
                }
            }
        }
        Ok(LinkContext { anchor, links })
    }

    /// This context's anchor
    pub fn anchor(&self) -> &Uri<String> {
        &self.anchor
    }

    /// Add a link to this context.
    ///
    /// Will raise an error if the same link (target and rel)
    /// is already present in the context.
    pub fn add_link(&mut self, link: Link) -> Result<(), LinksetError> {
        for l in &self.links {
            if l.target() == link.target() && l.rel() == link.rel() {
                return Err(LinksetError::DuplicateLink(
                    self.anchor.clone(),
                    link.rel().clone(),
                    link.target().clone(),
                ));
            }
        }
        self.links.push(link);
        Ok(())
    }

    /// Remove a link from this context, preserving the order of the other links.
    pub fn del_link(&mut self, idx: usize) {
        let len = self.links.len();
        for i in idx + 1..len {
            self.links.swap(i - 1, i);
        }
        self.links.pop();
    }

    /// Merge two contexts having the same anchor.
    ///
    /// This method will panic if the contexts have different anchors,
    /// and will raise an error if the same link (target, rel and anchor)
    /// occurs more than one.
    pub fn merge(&mut self, other: LinkContext) -> Result<(), LinksetError> {
        assert_eq!(self.anchor, other.anchor);
        for link in other.links {
            self.add_link(link)?;
        }
        Ok(())
    }
}

impl PartialEq for LinkContext {
    fn eq(&self, other: &Self) -> bool {
        if self.anchor != other.anchor || self.links.len() != other.links.len() {
            return false;
        }
        'outer: for ls in &self.links {
            for lo in &other.links {
                if ls == lo {
                    continue 'outer;
                }
            }
            return false;
        }
        true
    }
}

impl std::ops::Deref for LinkContext {
    type Target = Vec<Link>;

    fn deref(&self) -> &Self::Target {
        &self.links
    }
}

impl std::ops::Index<usize> for LinkContext {
    type Output = Link;

    fn index(&self, index: usize) -> &Self::Output {
        &self.links[index]
    }
}

impl std::ops::IndexMut<usize> for LinkContext {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.links[index]
    }
}

impl IntoIterator for LinkContext {
    type Item = Link;

    type IntoIter = std::vec::IntoIter<Link>;

    fn into_iter(self) -> Self::IntoIter {
        self.links.into_iter()
    }
}

impl<'a> IntoIterator for &'a LinkContext {
    type Item = &'a Link;

    type IntoIter = std::slice::Iter<'a, Link>;

    fn into_iter(self) -> Self::IntoIter {
        self.links.iter()
    }
}
