use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::{attr, attr_non_empty, has_attr, opening_tag_span};
use std::collections::HashSet;
use tl::{HTMLTag, Parser, VDom};

/// Ogni controllo di form (`<input>`, `<select>`, `<textarea>`) deve avere
/// un'etichetta associata. È valido un `<label for>`, un `<label>` che lo
/// avvolge, oppure `aria-label`/`aria-labelledby`/`title`. Senza etichetta lo
/// screen reader non sa cosa rappresenti il campo.
pub struct LabelControl;

/// Tipi di `<input>` che non richiedono un'etichetta esplicita: `hidden` non è
/// visibile; i bottoni ricavano il nome da `value`/`alt`.
const INPUT_TYPES_WITHOUT_LABEL: &[&str] =
    &["hidden", "submit", "button", "reset", "image"];

impl Rule for LabelControl {
    fn id(&self) -> &'static str {
        "label-control"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            summary: "Every form control has an associated label",
            help: "Associate a <label for>, wrap the control in a <label>, or add an aria-label.",
            example_bad: r#"<input type="email" name="email">"#,
            example_good: r#"<label for="email">Email</label><input id="email" type="email">"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/label",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();

        // Una passata sui <label>: raccogliamo gli id referenziati via `for` e
        // gli offset dei controlli avvolti da un <label> (associazione implicita).
        let mut labeled_ids: HashSet<String> = HashSet::new();
        let mut wrapped: HashSet<usize> = HashSet::new();
        if let Some(labels) = dom.query_selector("label") {
            for h in labels {
                let Some(label) = h.get(parser).and_then(|n| n.as_tag()) else {
                    continue;
                };
                if let Some(target) = attr(label, "for") {
                    let v = target.trim();
                    if !v.is_empty() {
                        labeled_ids.insert(v.to_string());
                    }
                }
                for node in label.children().all(parser) {
                    let Some(t) = node.as_tag() else { continue };
                    if is_control(t) {
                        wrapped.insert(t.boundaries(parser).0);
                    }
                }
            }
        }

        let mut out = Vec::new();
        for selector in ["input", "select", "textarea"] {
            let Some(controls) = dom.query_selector(selector) else {
                continue;
            };
            for h in controls {
                let Some(tag) = h.get(parser).and_then(|n| n.as_tag()) else {
                    continue;
                };
                if !needs_label(tag) || has_label(tag, parser, &labeled_ids, &wrapped) {
                    continue;
                }
                out.push(Finding::new(
                    self.id(),
                    Severity::Warn,
                    "form control has no associated <label>",
                    Some(opening_tag_span(tag, parser, src)),
                ));
            }
        }
        out
    }
}

/// Vero se il tag è un controllo di form (per il rilevamento dell'avvolgimento).
fn is_control(tag: &HTMLTag<'_>) -> bool {
    let name = tag.name().as_bytes();
    name.eq_ignore_ascii_case(b"input")
        || name.eq_ignore_ascii_case(b"select")
        || name.eq_ignore_ascii_case(b"textarea")
}

/// Vero se il controllo richiede un'etichetta esplicita (esclude gli `<input>`
/// che ricavano il nome altrove o non sono visibili).
fn needs_label(tag: &HTMLTag<'_>) -> bool {
    if !tag.name().as_bytes().eq_ignore_ascii_case(b"input") {
        return true; // select / textarea
    }
    let ty = attr(tag, "type").map_or_else(|| "text".to_string(), |v| v.trim().to_ascii_lowercase());
    !INPUT_TYPES_WITHOUT_LABEL.contains(&ty.as_str())
}

/// Vero se il controllo ha un nome accessibile da etichetta o attributo ARIA.
fn has_label(
    tag: &HTMLTag<'_>,
    parser: &Parser<'_>,
    labeled_ids: &HashSet<String>,
    wrapped: &HashSet<usize>,
) -> bool {
    if attr_non_empty(tag, "aria-label")
        || has_attr(tag, "aria-labelledby")
        || attr_non_empty(tag, "title")
    {
        return true;
    }
    if attr(tag, "id").is_some_and(|id| labeled_ids.contains(id.trim())) {
        return true;
    }
    wrapped.contains(&tag.boundaries(parser).0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        LabelControl.check(&dom, src)
    }

    #[test]
    fn label_for_ok() {
        assert!(
            check(r#"<label for="e">Email</label><input id="e" type="email">"#).is_empty()
        );
    }

    #[test]
    fn label_che_avvolge_ok() {
        assert!(check(r#"<label>Email <input type="email"></label>"#).is_empty());
    }

    #[test]
    fn aria_label_ok() {
        assert!(check(r#"<input type="text" aria-label="Cerca">"#).is_empty());
    }

    #[test]
    fn senza_label_warn() {
        assert_eq!(check(r#"<input type="email" name="email">"#).len(), 1);
        assert_eq!(check("<select><option>a</option></select>").len(), 1);
        assert_eq!(check("<textarea></textarea>").len(), 1);
    }

    #[test]
    fn input_hidden_e_submit_ignorati() {
        assert!(check(r#"<input type="hidden" name="t">"#).is_empty());
        assert!(check(r#"<input type="submit" value="Invia">"#).is_empty());
    }
}
