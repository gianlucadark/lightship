use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::{attr, opening_tag_span};
use std::collections::HashSet;
use tl::VDom;

/// Gli `id` devono essere unici nel documento: duplicati rompono ancoraggi,
/// `getElementById`, `label[for]` e i riferimenti ARIA.
pub struct DuplicateId;

impl Rule for DuplicateId {
    fn id(&self) -> &'static str {
        "duplicate-id"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Error,
            summary: "Element ids are unique in the document",
            help: "Make every id unique; use a class for shared styling or groups.",
            example_bad: r#"<p id="x"></p><span id="x"></span>"#,
            example_good: r#"<p id="intro"></p><span id="cta"></span>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Global_attributes/id",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(elems) = dom.query_selector("[id]") else {
            return Vec::new();
        };
        let mut seen: HashSet<String> = HashSet::new();
        let mut findings = Vec::new();

        for tag in elems.filter_map(|h| h.get(parser)?.as_tag()) {
            let Some(id) = attr(tag, "id") else {
                continue;
            };
            let id = id.trim();
            if id.is_empty() {
                continue;
            }
            if !seen.insert(id.to_string()) {
                findings.push(Finding::new(
                    self.id(),
                    Severity::Error,
                    format!("duplicate id: \"{id}\""),
                    Some(opening_tag_span(tag, parser, src)),
                ));
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        let mut findings = DuplicateId.check(&dom, src);
        findings.iter_mut().for_each(|f| f.source = src.into());
        findings
    }

    #[test]
    fn segnala_solo_le_occorrenze_oltre_la_prima() {
        let f =
            check(r#"<html><body><p id="x"></p><span id="x"></span><i id="y"></i></body></html>"#);
        assert_eq!(f.len(), 1);
        // punta al secondo elemento, quello duplicato
        assert_eq!(f[0].snippet(), Some(r#"<span id="x">"#));
    }

    #[test]
    fn id_unici_nessun_finding() {
        assert!(check(r#"<html><body><p id="a"></p><p id="b"></p></body></html>"#).is_empty());
    }
}
