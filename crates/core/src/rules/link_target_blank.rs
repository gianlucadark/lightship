use crate::finding::{Finding, Severity};
use crate::fix::{Edit, Fix};
use crate::meta::{Category, RuleMeta};
use crate::rule::Rule;
use crate::util::{attr, attr_insert_pos, opening_tag_span};
use tl::{HTMLTag, VDom};

/// Un `<a target="_blank">` dovrebbe avere `rel="noopener"` (o `noreferrer`): la
/// nuova pagina, altrimenti, può manipolare quella di origine via `window.opener`
/// e perde l'isolamento di processo (rischio sicurezza/performance).
pub struct LinkTargetBlank;

impl Rule for LinkTargetBlank {
    fn id(&self) -> &'static str {
        "link-target-blank"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Security,
            summary: "<a target=\"_blank\"> sets rel=\"noopener\"",
            help: "Add rel=\"noopener\" (or noreferrer) to target=\"_blank\" links.",
            example_bad: r#"<a href="https://x.com" target="_blank">x</a>"#,
            example_good: r#"<a href="https://x.com" target="_blank" rel="noopener">x</a>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Attributes/rel/noopener",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(links) = dom.query_selector("a") else {
            return Vec::new();
        };
        links
            .filter_map(|h| h.get(parser)?.as_tag())
            .filter(|t| attr(t, "target").is_some_and(|v| v.trim().eq_ignore_ascii_case("_blank")))
            .filter(|t| !has_safe_rel(t))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Warn,
                    "target=\"_blank\" link is missing rel=\"noopener\"",
                    Some(opening_tag_span(tag, parser, src)),
                )
            })
            .collect()
    }

    fn fixes(&self, dom: &VDom<'_>, src: &str) -> Vec<Fix> {
        let parser = dom.parser();
        let Some(links) = dom.query_selector("a") else {
            return Vec::new();
        };
        links
            .filter_map(|h| h.get(parser)?.as_tag())
            .filter(|t| attr(t, "target").is_some_and(|v| v.trim().eq_ignore_ascii_case("_blank")))
            .filter(|t| !has_safe_rel(t))
            .map(|tag| {
                let span = opening_tag_span(tag, parser, src);
                let at = attr_insert_pos(src, span);
                Fix::new(
                    self.id(),
                    "add rel=\"noopener\" to this target=\"_blank\" link",
                    "rel=\"noopener\"",
                    Edit::insert(at, " rel=\"noopener\""),
                )
            })
            .collect()
    }
}

/// Vero se `rel` contiene il token `noopener` o `noreferrer` (case-insensitive).
fn has_safe_rel(tag: &HTMLTag<'_>) -> bool {
    attr(tag, "rel").is_some_and(|rel| {
        rel.split_whitespace().any(|tok| {
            tok.eq_ignore_ascii_case("noopener") || tok.eq_ignore_ascii_case("noreferrer")
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        LinkTargetBlank.check(&dom, src)
    }

    #[test]
    fn con_noopener_ok() {
        assert!(check(r#"<a href="/x" target="_blank" rel="noopener">x</a>"#).is_empty());
    }

    #[test]
    fn noreferrer_ok() {
        assert!(check(r#"<a href="/x" target="_blank" rel="noreferrer">x</a>"#).is_empty());
    }

    #[test]
    fn senza_rel_warn() {
        assert_eq!(check(r#"<a href="/x" target="_blank">x</a>"#).len(), 1);
    }

    #[test]
    fn target_self_ignorato() {
        assert!(check(r#"<a href="/x">x</a>"#).is_empty());
    }
}
