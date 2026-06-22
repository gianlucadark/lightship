use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::{attr, opening_tag_span};
use tl::VDom;

/// Intervallo consigliato per la meta description (caratteri).
const MIN_CHARS: usize = 50;
const MAX_CHARS: usize = 160;

/// La `<meta name="description">`, quando presente e non vuota, dovrebbe stare
/// tra ~50 e ~160 caratteri: troppo corta è poco informativa, troppo lunga viene
/// troncata nelle SERP. Presenza/vuoto sono coperti da `meta-description`.
pub struct MetaDescriptionLength;

impl Rule for MetaDescriptionLength {
    fn id(&self) -> &'static str {
        "meta-description-length"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            summary: "The meta description is ~50–160 characters",
            help: "Aim for a 50–160 character description so it isn't truncated in search results.",
            example_bad: r#"<meta name="description" content="Shoes.">"#,
            example_good: r#"<meta name="description" content="Handmade leather shoes, crafted in Milan since 1965 — free shipping across the EU.">"#,
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

        let Some(tag) = meta else {
            return Vec::new();
        };
        let content = match attr(tag, "content") {
            Some(c) => c.trim().chars().count(),
            None => return Vec::new(),
        };
        // Una description vuota è compito di `meta-description`, non nostro.
        if content == 0 {
            return Vec::new();
        }

        let problem = if content < MIN_CHARS {
            Some(format!("meta description is short ({content} chars; aim for ≥{MIN_CHARS})"))
        } else if content > MAX_CHARS {
            Some(format!("meta description is long ({content} chars; keep ≤{MAX_CHARS})"))
        } else {
            None
        };

        problem
            .map(|message| {
                vec![Finding::new(
                    self.id(),
                    Severity::Warn,
                    message,
                    Some(opening_tag_span(tag, parser, src)),
                )]
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        MetaDescriptionLength.check(&dom, src)
    }

    fn meta(content: &str) -> String {
        format!(r#"<meta name="description" content="{content}">"#)
    }

    #[test]
    fn lunghezza_giusta_ok() {
        assert!(check(&meta(&"x".repeat(80))).is_empty());
    }

    #[test]
    fn troppo_corta_warn() {
        assert_eq!(check(&meta("Scarpe.")).len(), 1);
    }

    #[test]
    fn troppo_lunga_warn() {
        assert_eq!(check(&meta(&"x".repeat(200))).len(), 1);
    }

    #[test]
    fn assente_o_vuota_non_segnalata() {
        assert!(check("<head></head>").is_empty());
        assert!(check(&meta("")).is_empty());
    }
}
