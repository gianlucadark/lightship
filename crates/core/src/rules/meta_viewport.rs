use crate::finding::{Finding, Severity};
use crate::rule::Rule;
use crate::util::attr;
use tl::VDom;

/// Senza `<meta name="viewport">` la pagina non è responsive sul mobile.
pub struct MetaViewport;

impl Rule for MetaViewport {
    fn id(&self) -> &'static str {
        "meta-viewport"
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
            "manca <meta name=\"viewport\"> (pagina non responsive)",
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
            check(r#"<html><head><meta name="viewport" content="width=device-width"></head></html>"#)
                .is_empty()
        );
    }

    #[test]
    fn assente_warn() {
        assert_eq!(check("<html><head></head></html>").len(), 1);
    }
}
