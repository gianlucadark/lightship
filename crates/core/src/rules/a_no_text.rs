use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::Rule;
use crate::util::{has_accessible_name, has_attr, is_a11y_hidden, opening_tag_span};
use tl::VDom;

/// Un link (`<a href>`) deve avere un nome accessibile: testo, `aria-label`,
/// `title` o un'immagine con `alt`. I link "vuoti" sono inutilizzabili da screen
/// reader e tastiera.
pub struct ANoText;

impl Rule for ANoText {
    fn id(&self) -> &'static str {
        "a-no-text"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Accessibility,
            summary: "Every <a href> has an accessible name",
            help: "Give the link a name: visible text, aria-label, title, or an <img alt>.",
            example_bad: r#"<a href="/x"></a>"#,
            example_good: r#"<a href="/x" aria-label="Go to home"></a>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/a#accessibility",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(links) = dom.query_selector("a") else {
            return Vec::new();
        };
        links
            .filter_map(|h| h.get(parser)?.as_tag())
            .filter(|tag| has_attr(tag, "href"))
            .filter(|tag| !is_a11y_hidden(tag))
            .filter(|tag| !has_accessible_name(tag, parser))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Warn,
                    "link has no text or accessible name",
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

    #[test]
    fn aria_hidden_non_segnalato() {
        // Link nascosto agli screen reader: nessun nome richiesto.
        assert!(check(r#"<a href="/x" aria-hidden="true"></a>"#).is_empty());
    }
}
