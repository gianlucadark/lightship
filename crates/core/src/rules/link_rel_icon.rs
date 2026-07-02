use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::attr;
use tl::VDom;

/// Senza un `<link rel="icon">` esplicito il browser tenta `/favicon.ico` alla
/// cieca (404 nei log se manca) e la pagina si presenta anonima nelle tab, nei
/// preferiti e nei risultati di ricerca.
pub struct LinkRelIcon;

impl Rule for LinkRelIcon {
    fn id(&self) -> &'static str {
        "link-rel-icon"
    }

    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Seo,
            summary: "a favicon is declared with <link rel=\"icon\">",
            help: "Add <link rel=\"icon\" href=\"/favicon.ico\"> (and ideally an apple-touch-icon) in <head>.",
            example_bad: "<head><title>Home</title></head>",
            example_good: r#"<link rel="icon" href="/favicon.svg" type="image/svg+xml">"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Attributes/rel#icon",
        }
    }

    fn check(&self, dom: &VDom<'_>, _src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let present = dom.query_selector("link").is_some_and(|mut it| {
            it.any(|h| {
                h.get(parser)
                    .and_then(|n| n.as_tag())
                    .and_then(|t| attr(t, "rel"))
                    .is_some_and(|rel| {
                        rel.split_whitespace().any(|tok| {
                            tok.eq_ignore_ascii_case("icon")
                                || tok.eq_ignore_ascii_case("apple-touch-icon")
                        })
                    })
            })
        });
        if present {
            return Vec::new();
        }
        vec![Finding::new(
            self.id(),
            Severity::Warn,
            "missing <link rel=\"icon\"> (no favicon declared)",
            None,
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        LinkRelIcon.check(&dom, src)
    }

    #[test]
    fn icon_ok() {
        assert!(check(r#"<head><link rel="icon" href="/favicon.svg"></head>"#).is_empty());
    }

    #[test]
    fn shortcut_icon_ok() {
        assert!(check(r#"<head><link rel="shortcut icon" href="/favicon.ico"></head>"#).is_empty());
    }

    #[test]
    fn apple_touch_icon_ok() {
        assert!(check(r#"<head><link rel="apple-touch-icon" href="/icon.png"></head>"#).is_empty());
    }

    #[test]
    fn assente_warn() {
        assert_eq!(check(r#"<head><link rel="stylesheet" href="/a.css"></head>"#).len(), 1);
    }
}
