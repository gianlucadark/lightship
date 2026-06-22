use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::attr;
use tl::VDom;

/// Senza `<meta name="viewport">` la pagina non è responsive sul mobile.
pub struct MetaViewport;

impl Rule for MetaViewport {
    fn id(&self) -> &'static str {
        "meta-viewport"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            summary: "<meta name=\"viewport\"> is present",
            help: "Add <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"> for responsive pages.",
            example_bad: "<head><meta charset=\"utf-8\"></head>",
            example_good: r#"<meta name="viewport" content="width=device-width, initial-scale=1">"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Viewport_meta_tag",
        }
    }

    fn check(&self, dom: &VDom<'_>, _src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let present = dom.query_selector("meta").is_some_and(|mut it| {
            it.any(|h| {
                h.get(parser)
                    .and_then(|n| n.as_tag())
                    .and_then(|t| attr(t, "name"))
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("viewport"))
            })
        });
        if present {
            return Vec::new();
        }
        vec![Finding::new(
            self.id(),
            Severity::Warn,
            "missing <meta name=\"viewport\"> (page is not responsive)",
            None,
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        MetaViewport.check(&dom, src)
    }

    #[test]
    fn presente_ok() {
        assert!(
            check(
                r#"<html><head><meta name="viewport" content="width=device-width"></head></html>"#
            )
            .is_empty()
        );
    }

    #[test]
    fn assente_warn() {
        assert_eq!(check("<html><head></head></html>").len(), 1);
    }
}
