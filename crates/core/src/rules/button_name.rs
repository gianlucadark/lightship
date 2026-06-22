use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::{has_accessible_name, is_a11y_hidden, opening_tag_span};
use tl::VDom;

/// Ogni `<button>` deve avere un nome accessibile: testo visibile, `aria-label`,
/// `title` o un'immagine discendente con `alt`. Un bottone "muto" è inutilizzabile
/// da screen reader e tastiera (è frequente con i bottoni a sola icona).
pub struct ButtonName;

impl Rule for ButtonName {
    fn id(&self) -> &'static str {
        "button-name"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            summary: "Every <button> has an accessible name",
            help: "Give the button text, an aria-label, or an <img alt> (common with icon buttons).",
            example_bad: r#"<button><svg>…</svg></button>"#,
            example_good: r#"<button aria-label="Close">✕</button>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/button#accessibility",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(buttons) = dom.query_selector("button") else {
            return Vec::new();
        };
        buttons
            .filter_map(|h| h.get(parser)?.as_tag())
            .filter(|tag| !is_a11y_hidden(tag))
            .filter(|tag| !has_accessible_name(tag, parser))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Warn,
                    "<button> has no text or accessible name",
                    Some(opening_tag_span(tag, parser, src)),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        ButtonName.check(&dom, src)
    }

    #[test]
    fn con_testo_ok() {
        assert!(check("<button>Salva</button>").is_empty());
    }

    #[test]
    fn icona_con_aria_label_ok() {
        assert!(check(r#"<button aria-label="Chiudi"><svg></svg></button>"#).is_empty());
    }

    #[test]
    fn icona_muta_warn() {
        assert_eq!(check("<button><svg></svg></button>").len(), 1);
    }

    #[test]
    fn aria_hidden_non_segnalato() {
        assert!(check(r#"<button aria-hidden="true"></button>"#).is_empty());
    }
}
