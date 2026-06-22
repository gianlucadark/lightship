use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::has_attr;
use tl::VDom;

/// Ogni pagina dovrebbe dichiarare la codifica con `<meta charset>` (idealmente
/// utf-8) come primo elemento del `<head>`.
pub struct MetaCharset;

impl Rule for MetaCharset {
    fn id(&self) -> &'static str {
        "meta-charset"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
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
