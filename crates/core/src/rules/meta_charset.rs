use crate::finding::{Finding, Severity};
use crate::fix::{Edit, Fix};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::{has_attr, head_open_end};
use tl::VDom;

/// Ogni pagina dovrebbe dichiarare la codifica con `<meta charset>` (idealmente
/// utf-8) come primo elemento del `<head>`.
pub struct MetaCharset;

impl Rule for MetaCharset {
    fn id(&self) -> &'static str {
        "meta-charset"
    }

    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Seo,
            summary: "<meta charset> is present",
            help: "Add <meta charset=\"utf-8\"> as the first element of <head>.",
            example_bad: "<head><title>Page</title></head>",
            example_good: r#"<head><meta charset="utf-8"></head>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/meta#charset",
        }
    }

    fn check(&self, dom: &VDom<'_>, _src: &str) -> Vec<Finding> {
        // Query per tag `meta` + check case-insensitive: React/Next emette `charSet`.
        let parser = dom.parser();
        let present = dom.query_selector("meta").is_some_and(|mut it| {
            it.any(|h| {
                h.get(parser)
                    .and_then(|n| n.as_tag())
                    .is_some_and(|t| has_attr(t, "charset"))
            })
        });
        if present {
            return Vec::new();
        }
        vec![Finding::new(
            self.id(),
            Severity::Warn,
            "missing <meta charset>",
            None,
        )]
    }

    fn fixes(&self, dom: &VDom<'_>, src: &str) -> Vec<Fix> {
        let parser = dom.parser();
        let present = dom.query_selector("meta").is_some_and(|mut it| {
            it.any(|h| {
                h.get(parser)
                    .and_then(|n| n.as_tag())
                    .is_some_and(|t| has_attr(t, "charset"))
            })
        });
        if present {
            return Vec::new();
        }
        // Inseriamo come primo figlio di <head>. Senza <head> non proponiamo nulla.
        let Some(at) = head_open_end(dom, parser, src) else {
            return Vec::new();
        };
        vec![Fix::new(
            self.id(),
            "insert <meta charset=\"utf-8\"> as the first <head> child",
            "<meta charset=\"utf-8\">",
            Edit::insert(at, "<meta charset=\"utf-8\">"),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        MetaCharset.check(&dom, src)
    }

    #[test]
    fn presente_ok() {
        assert!(check(r#"<html><head><meta charset="utf-8"></head></html>"#).is_empty());
    }

    #[test]
    fn charset_camelcase_ok() {
        // React/Next emette `charSet`: gli attributi HTML sono case-insensitive.
        assert!(check(r#"<html><head><meta charSet="utf-8"/></head></html>"#).is_empty());
    }

    #[test]
    fn assente_warn() {
        assert_eq!(check("<html><head></head></html>").len(), 1);
    }
}
