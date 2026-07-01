use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::opening_tag_span;
use tl::{HTMLTag, VDom};

/// I livelli di heading non dovrebbero saltare: dopo un `<h2>` ci si aspetta un
/// `<h3>`, non direttamente un `<h4>`. I salti rompono la struttura del documento
/// su cui si basano screen reader e SEO per costruire l'outline della pagina.
pub struct HeadingOrder;

impl Rule for HeadingOrder {
    fn id(&self) -> &'static str {
        "heading-order"
    }

    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Accessibility,
            summary: "Heading levels increase by one, without skipping",
            help: "Don't jump heading levels (e.g. <h2> then <h4>); step down one level at a time.",
            example_bad: "<h2>Section</h2><h4>Detail</h4>",
            example_good: "<h2>Section</h2><h3>Detail</h3>",
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/Heading_Elements",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        // Headings in ordine di documento (i nodi di `tl` sono in ordine di parse).
        let headings = dom
            .nodes()
            .iter()
            .filter_map(|n| n.as_tag())
            .filter_map(|t| heading_level(t).map(|lvl| (lvl, t)));

        let mut out = Vec::new();
        let mut prev: Option<u8> = None;
        for (level, tag) in headings {
            if let Some(p) = prev
                && level > p + 1
            {
                out.push(Finding::new(
                    self.id(),
                    Severity::Warn,
                    format!("heading level skipped from h{p} to h{level}"),
                    Some(opening_tag_span(tag, parser, src)),
                ));
            }
            prev = Some(level);
        }
        out
    }
}

/// Livello (1–6) se il tag è un heading `h1`…`h6`, altrimenti `None`.
fn heading_level(tag: &HTMLTag<'_>) -> Option<u8> {
    let name = tag.name().as_bytes();
    if name.len() == 2 && (name[0] == b'h' || name[0] == b'H') && name[1].is_ascii_digit() {
        let level = name[1] - b'0';
        (1..=6).contains(&level).then_some(level)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        HeadingOrder.check(&dom, src)
    }

    #[test]
    fn ordine_corretto_ok() {
        assert!(check("<h1>a</h1><h2>b</h2><h3>c</h3><h2>d</h2>").is_empty());
    }

    #[test]
    fn salto_di_livello_warn() {
        assert_eq!(check("<h2>a</h2><h4>b</h4>").len(), 1);
    }

    #[test]
    fn primo_heading_non_penalizzato() {
        // Partire da h3 non è un salto: manca un heading precedente.
        assert!(check("<h3>a</h3><h4>b</h4>").is_empty());
    }
}
