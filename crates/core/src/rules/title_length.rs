use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::opening_tag_span;
use tl::VDom;

/// Limite oltre il quale un `<title>` viene troncato nei risultati di ricerca.
const MAX_TITLE_CHARS: usize = 60;

/// Il `<title>` non dovrebbe superare ~60 caratteri, altrimenti viene troncato
/// nelle SERP. La presenza/vuoto è coperta da `title-present`: qui valutiamo solo
/// la lunghezza di un titolo già presente.
pub struct TitleLength;

impl Rule for TitleLength {
    fn id(&self) -> &'static str {
        "title-length"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            summary: "The <title> is at most ~60 characters",
            help: "Keep the <title> under ~60 characters so it isn't truncated in search results.",
            example_bad: "<title>The complete and exhaustive guide to absolutely everything we sell</title>",
            example_good: "<title>Handmade Leather Shoes · Acme</title>",
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/title",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(tag) = dom
            .query_selector("title")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser)?.as_tag())
        else {
            return Vec::new();
        };

        let text = tag.inner_text(parser);
        let len = text.trim().chars().count();
        if len > MAX_TITLE_CHARS {
            vec![Finding::new(
                self.id(),
                Severity::Warn,
                format!("<title> is {len} characters; keep it under ~{MAX_TITLE_CHARS}"),
                Some(opening_tag_span(tag, parser, src)),
            )]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        TitleLength.check(&dom, src)
    }

    #[test]
    fn corto_ok() {
        assert!(check("<title>About · Acme</title>").is_empty());
    }

    #[test]
    fn lungo_warn() {
        let long = "x".repeat(80);
        assert_eq!(check(&format!("<title>{long}</title>")).len(), 1);
    }

    #[test]
    fn assente_non_segnalato() {
        // La presenza è compito di title-present.
        assert!(check("<head></head>").is_empty());
    }
}
