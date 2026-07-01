use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::{attr, attr_non_empty, opening_tag_span};
use tl::VDom;

/// Il `<link rel="canonical">` indica l'URL preferito della pagina. Non lo
/// pretendiamo (molte pagine non ne hanno bisogno), ma segnaliamo gli errori
/// concreti: **più di un** canonical, o un canonical con `href` vuoto — entrambi
/// confondono i motori di ricerca.
pub struct CanonicalLink;

impl Rule for CanonicalLink {
    fn id(&self) -> &'static str {
        "canonical-link"
    }

    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Seo,
            summary: "At most one <link rel=\"canonical\"> with a non-empty href",
            help: "Keep a single canonical link with an absolute href; remove duplicates.",
            example_bad: r#"<link rel="canonical" href=""><link rel="canonical" href="/a">"#,
            example_good: r#"<link rel="canonical" href="https://acme.com/a">"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Attributes/rel/canonical",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let canonicals: Vec<_> = dom
            .query_selector("link")
            .map(|it| {
                it.filter_map(|h| h.get(parser)?.as_tag())
                    .filter(|t| {
                        attr(t, "rel").is_some_and(|v| v.trim().eq_ignore_ascii_case("canonical"))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut out = Vec::new();
        // href vuoto su ognuno.
        for tag in &canonicals {
            if !attr_non_empty(tag, "href") {
                out.push(Finding::new(
                    self.id(),
                    Severity::Warn,
                    "<link rel=\"canonical\"> has an empty href",
                    Some(opening_tag_span(tag, parser, src)),
                ));
            }
        }
        // Duplicati: segnaliamo dal secondo in poi.
        for tag in canonicals.iter().skip(1) {
            out.push(Finding::new(
                self.id(),
                Severity::Warn,
                "duplicate <link rel=\"canonical\"> on the page",
                Some(opening_tag_span(tag, parser, src)),
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        CanonicalLink.check(&dom, src)
    }

    #[test]
    fn uno_valido_ok() {
        assert!(check(r#"<link rel="canonical" href="https://a.com/x">"#).is_empty());
    }

    #[test]
    fn assente_ok() {
        // Nessun canonical non è un errore.
        assert!(check("<head></head>").is_empty());
    }

    #[test]
    fn href_vuoto_warn() {
        assert_eq!(check(r#"<link rel="canonical" href="">"#).len(), 1);
    }

    #[test]
    fn duplicato_warn() {
        let f = check(r#"<link rel="canonical" href="/a"><link rel="canonical" href="/b">"#);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("duplicate"));
    }
}
