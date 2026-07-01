use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::{attr, collect_ids, opening_tag_span};
use tl::VDom;

/// Gli attributi `aria-labelledby` e `aria-describedby` contengono una lista di
/// `id` (separati da spazi) che devono esistere nel documento: se un id manca,
/// il nome/descrizione accessibile non viene calcolato e l'elemento resta muto
/// per gli screen reader.
pub struct AriaRefTarget;

/// Attributi ARIA che referenziano id di altri elementi.
const IDREF_ATTRS: &[&str] = &["aria-labelledby", "aria-describedby"];

impl Rule for AriaRefTarget {
    fn id(&self) -> &'static str {
        "aria-ref-target"
    }

    // Come label-for-target: risoluzione a livello di documento intero.
    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Accessibility,
            summary: "aria-labelledby/aria-describedby reference existing ids",
            help: "Point aria-labelledby/aria-describedby at ids that exist in the page.",
            example_bad: r#"<button aria-labelledby="lbl">✕</button>"#,
            example_good: r#"<span id="lbl">Close</span><button aria-labelledby="lbl">✕</button>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/Accessibility/ARIA/Attributes/aria-labelledby",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let ids = collect_ids(dom, parser);
        let mut out = Vec::new();

        for &aria in IDREF_ATTRS {
            let selector = format!("[{aria}]");
            let Some(els) = dom.query_selector(&selector) else {
                continue;
            };
            for tag in els.filter_map(|h| h.get(parser)?.as_tag()) {
                let Some(value) = attr(tag, aria) else {
                    continue;
                };
                let missing: Vec<&str> = value
                    .split_whitespace()
                    .filter(|token| !ids.contains(*token))
                    .collect();
                if missing.is_empty() {
                    continue;
                }
                out.push(Finding::new(
                    self.id(),
                    Severity::Warn,
                    format!("{aria} references missing id(s): {}", missing.join(", ")),
                    Some(opening_tag_span(tag, parser, src)),
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        AriaRefTarget.check(&dom, src)
    }

    #[test]
    fn riferimento_esistente_ok() {
        assert!(
            check(r#"<span id="l">Close</span><button aria-labelledby="l">x</button>"#).is_empty()
        );
    }

    #[test]
    fn riferimento_mancante_warn() {
        assert_eq!(check(r#"<button aria-labelledby="l">x</button>"#).len(), 1);
    }

    #[test]
    fn lista_con_un_id_mancante_warn() {
        let f = check(r#"<span id="a"></span><button aria-describedby="a b">x</button>"#);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains('b'));
    }
}
