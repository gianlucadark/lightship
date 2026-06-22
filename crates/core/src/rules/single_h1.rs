use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::opening_tag_span;
use tl::VDom;

/// Una pagina dovrebbe avere **esattamente un** `<h1>`: è il titolo principale
/// usato da motori di ricerca e tecnologie assistive per capire l'argomento.
pub struct SingleH1;

impl Rule for SingleH1 {
    fn id(&self) -> &'static str {
        "single-h1"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            summary: "The page has exactly one <h1>",
            help: "Use a single <h1> as the page's main heading; demote the others to <h2>…<h6>.",
            example_bad: "<h1>Acme</h1><h1>Welcome</h1>",
            example_good: "<h1>Welcome to Acme</h1><h2>Products</h2>",
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/Heading_Elements",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let h1s: Vec<_> = dom
            .query_selector("h1")
            .map(|it| it.filter_map(|h| h.get(parser)?.as_tag()).collect())
            .unwrap_or_default();

        match h1s.len() {
            0 => vec![Finding::new(
                self.id(),
                Severity::Warn,
                "page has no <h1> heading",
                None,
            )],
            1 => Vec::new(),
            n => {
                // Segnaliamo a partire dal secondo <h1>: il primo è quello "buono".
                h1s.iter()
                    .skip(1)
                    .map(|tag| {
                        Finding::new(
                            self.id(),
                            Severity::Warn,
                            format!("page has {n} <h1> headings; keep only one"),
                            Some(opening_tag_span(tag, parser, src)),
                        )
                    })
                    .collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        SingleH1.check(&dom, src)
    }

    #[test]
    fn uno_solo_ok() {
        assert!(check("<h1>Ciao</h1><h2>Sezione</h2>").is_empty());
    }

    #[test]
    fn zero_warn() {
        assert_eq!(check("<h2>Sezione</h2>").len(), 1);
    }

    #[test]
    fn multipli_warn_uno_per_extra() {
        // Tre <h1> ⇒ due finding (sul 2° e 3°).
        assert_eq!(check("<h1>a</h1><h1>b</h1><h1>c</h1>").len(), 2);
    }
}
