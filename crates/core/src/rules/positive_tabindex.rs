use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::Rule;
use crate::util::{attr, opening_tag_span};
use tl::VDom;

/// Un `tabindex` positivo forza un ordine di tabulazione esplicito che quasi
/// sempre confonde: scavalca l'ordine naturale del DOM e va tenuto sincronizzato
/// a mano. Le best practice raccomandano solo `tabindex="0"` (focalizzabile
/// nell'ordine naturale) o `tabindex="-1"` (focalizzabile solo via script).
pub struct PositiveTabindex;

impl Rule for PositiveTabindex {
    fn id(&self) -> &'static str {
        "positive-tabindex"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Accessibility,
            summary: "No element uses a positive tabindex",
            help: "Use tabindex=\"0\" or \"-1\"; a positive tabindex breaks the natural tab order.",
            example_bad: r#"<button tabindex="3">Send</button>"#,
            example_good: r#"<button tabindex="0">Send</button>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Global_attributes/tabindex",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(els) = dom.query_selector("[tabindex]") else {
            return Vec::new();
        };
        els.filter_map(|h| h.get(parser)?.as_tag())
            .filter(|tag| is_positive(attr(tag, "tabindex").as_deref()))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Warn,
                    "positive tabindex overrides the natural tab order",
                    Some(opening_tag_span(tag, parser, src)),
                )
            })
            .collect()
    }
}

/// Vero se il valore di `tabindex` è un intero strettamente positivo.
fn is_positive(value: Option<&str>) -> bool {
    value
        .and_then(|v| v.trim().parse::<i32>().ok())
        .is_some_and(|n| n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        PositiveTabindex.check(&dom, src)
    }

    #[test]
    fn zero_e_negativo_ok() {
        assert!(check(r#"<div tabindex="0"></div><div tabindex="-1"></div>"#).is_empty());
    }

    #[test]
    fn positivo_warn() {
        assert_eq!(check(r#"<button tabindex="3">x</button>"#).len(), 1);
    }

    #[test]
    fn non_numerico_ignorato() {
        assert!(check(r#"<div tabindex="abc"></div>"#).is_empty());
    }
}
