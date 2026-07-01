use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::{attr, collect_ids, opening_tag_span};
use tl::VDom;

/// Un `<label for="x">` deve puntare a un elemento con `id="x"` esistente:
/// altrimenti l'etichetta non è associata ad alcun controllo e lo screen reader
/// non sa a cosa si riferisce. È un errore comune dopo un refactor degli id.
pub struct LabelForTarget;

impl Rule for LabelForTarget {
    fn id(&self) -> &'static str {
        "label-for-target"
    }

    // Serve l'intero documento per risolvere gli id: su un frammento il target
    // potrebbe vivere altrove, quindi evitiamo i falsi positivi.
    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Accessibility,
            summary: "Every <label for> points to an existing element id",
            help: "Match the label's for to a control's id, or wrap the control in the <label>.",
            example_bad: r#"<label for="email">Email</label><input id="mail">"#,
            example_good: r#"<label for="email">Email</label><input id="email">"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/label#attr-for",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(labels) = dom.query_selector("label") else {
            return Vec::new();
        };
        let ids = collect_ids(dom, parser);

        labels
            .filter_map(|h| h.get(parser)?.as_tag())
            .filter_map(|tag| {
                let target = attr(tag, "for")?;
                let target = target.trim();
                if target.is_empty() || ids.contains(target) {
                    return None;
                }
                Some(Finding::new(
                    self.id(),
                    Severity::Warn,
                    format!("<label for=\"{target}\"> has no matching element id"),
                    Some(opening_tag_span(tag, parser, src)),
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        LabelForTarget.check(&dom, src)
    }

    #[test]
    fn target_esistente_ok() {
        assert!(check(r#"<label for="e">Email</label><input id="e">"#).is_empty());
    }

    #[test]
    fn target_mancante_warn() {
        assert_eq!(
            check(r#"<label for="e">Email</label><input id="mail">"#).len(),
            1
        );
    }

    #[test]
    fn label_senza_for_ignorata() {
        assert!(check(r#"<label>Email <input></label>"#).is_empty());
    }
}
