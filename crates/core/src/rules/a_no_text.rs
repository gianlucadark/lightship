use crate::finding::{Finding, Severity};
use crate::rule::Rule;
use crate::util::{has_accessible_name, has_attr, opening_tag_span};
use tl::VDom;

/// Un link (`<a href>`) deve avere un nome accessibile: testo, `aria-label`,
/// `title` o un'immagine con `alt`. I link "vuoti" sono inutilizzabili da screen
/// reader e tastiera.
pub struct ANoText;

impl Rule for ANoText {
    fn id(&self) -> &'static str {
        "a-no-text"
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(links) = dom.query_selector("a") else {
            return Vec::new();
        };
        links
            .filter_map(|h| h.get(parser)?.as_tag())
            .filter(|tag| has_attr(tag, "href"))
            .filter(|tag| !has_accessible_name(tag, parser))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Warn,
                    "link senza testo o nome accessibile",
                    Some(opening_tag_span(tag, parser, src)),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        let mut findings = ANoText.check(&dom, src);
        findings.iter_mut().for_each(|f| f.source = src.into());
        findings
    }

    #[test]
    fn con_testo_ok() {
        assert!(check(r#"<a href="/x">Chi siamo</a>"#).is_empty());
    }

    #[test]
    fn immagine_con_alt_ok() {
        assert!(check(r#"<a href="/x"><img src="i.png" alt="Home"></a>"#).is_empty());
    }

    #[test]
    fn vuoto_warn() {
        let f = check(r#"<a href="/x"></a>"#);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].snippet(), Some(r#"<a href="/x">"#));
    }
}
