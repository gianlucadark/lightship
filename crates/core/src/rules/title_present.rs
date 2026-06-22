use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::opening_tag_span;
use tl::VDom;

/// Deve esistere un `<title>` con testo non vuoto.
pub struct TitlePresent;

impl Rule for TitlePresent {
    fn id(&self) -> &'static str {
        "title-present"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Error,
            summary: "A <title> with non-empty text exists",
            help: "Add a descriptive, unique <title> in the page <head>.",
            example_bad: "<head></head>",
            example_good: "<head><title>About · Acme</title></head>",
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/title",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let title = dom
            .query_selector("title")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser)?.as_tag());

        match title {
            Some(tag) if tag.inner_text(parser).trim().is_empty() => vec![Finding::new(
                self.id(),
                Severity::Error,
                "<title> is present but empty",
                Some(opening_tag_span(tag, parser, src)),
            )],
            Some(_) => Vec::new(),
            None => vec![Finding::new(
                self.id(),
                Severity::Error,
                "missing a non-empty <title>",
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
        TitlePresent.check(&dom, src)
    }

    #[test]
    fn con_title_ok() {
        assert!(check("<html><head><title>Ciao</title></head></html>").is_empty());
    }

    #[test]
    fn senza_title_error() {
        let f = check("<html><head></head></html>");
        assert_eq!(f.len(), 1);
        assert!(f[0].span.is_none());
    }

    #[test]
    fn title_vuoto_error() {
        assert_eq!(
            check("<html><head><title>  </title></head></html>").len(),
            1
        );
    }
}
