use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::{attr, attr_non_empty, opening_tag_span};
use tl::VDom;

/// `<meta name="description">` deve esistere e avere `content` non vuoto: è la
/// descrizione usata da motori di ricerca e anteprime social.
pub struct MetaDescription;

impl Rule for MetaDescription {
    fn id(&self) -> &'static str {
        "meta-description"
    }

    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Seo,
            summary: "<meta name=\"description\"> with non-empty content is present",
            help: "Add a 50–160 character description: <meta name=\"description\" content=\"…\">.",
            example_bad: r#"<meta name="description" content="">"#,
            example_good: r#"<meta name="description" content="Handmade leather shoes, crafted in Milan.">"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/meta/name",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let meta = dom.query_selector("meta").and_then(|it| {
            it.filter_map(|h| h.get(parser)?.as_tag()).find(|t| {
                attr(t, "name").is_some_and(|v| v.trim().eq_ignore_ascii_case("description"))
            })
        });

        match meta {
            Some(tag) if !attr_non_empty(tag, "content") => vec![Finding::new(
                self.id(),
                Severity::Warn,
                "<meta name=\"description\"> has empty content",
                Some(opening_tag_span(tag, parser, src)),
            )],
            Some(_) => Vec::new(),
            None => vec![Finding::new(
                self.id(),
                Severity::Warn,
                "missing <meta name=\"description\">",
                None,
            )],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        MetaDescription.check(&dom, src)
    }

    #[test]
    fn con_content_ok() {
        assert!(
            check(r#"<html><head><meta name="description" content="Ciao"></head></html>"#)
                .is_empty()
        );
    }

    #[test]
    fn content_vuoto_warn() {
        assert_eq!(
            check(r#"<html><head><meta name="description" content="  "></head></html>"#).len(),
            1
        );
    }

    #[test]
    fn assente_warn() {
        assert_eq!(check("<html><head></head></html>").len(), 1);
    }
}
