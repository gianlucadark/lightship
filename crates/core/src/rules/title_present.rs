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
            summary: "Esiste un <title> con testo non vuoto",
            help: "Aggiungi un <title> descrittivo e unico nel <head> della pagina.",
            example_bad: "<head></head>",
            example_good: "<head><title>Chi siamo · Acme</title></head>",
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
                "<title> presente ma vuoto",
                Some(opening_tag_span(tag, parser, src)),
            )],
            Some(_) => Vec::new(),
            None => vec![Finding::new(
                self.id(),
                Severity::Error,
                "manca un <title> non vuoto",
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
