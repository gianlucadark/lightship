use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::Rule;
use crate::util::{attr, opening_tag_span};
use tl::VDom;

/// `<meta http-equiv="refresh">` ricarica o redirige la pagina senza che
/// l'utente possa fermarla: chi legge lentamente o usa uno screen reader perde
/// il contesto (fallimento WCAG F41). Il redirect va fatto lato server (301/308)
/// o con un link esplicito.
pub struct MetaRefresh;

impl Rule for MetaRefresh {
    fn id(&self) -> &'static str {
        "meta-refresh"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Error,
            category: Category::Accessibility,
            summary: "the page does not use <meta http-equiv=\"refresh\">",
            help: "Replace the meta refresh with a server-side redirect (301/308) or an explicit link the user can activate.",
            example_bad: r#"<meta http-equiv="refresh" content="5; url=/new">"#,
            example_good: r#"<a href="/new">Continue to the new page</a>"#,
            docs_url: "https://www.w3.org/TR/WCAG20-TECHS/F41.html",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(metas) = dom.query_selector("meta") else {
            return Vec::new();
        };
        metas
            .filter_map(|h| h.get(parser)?.as_tag())
            .filter(|t| {
                attr(t, "http-equiv").is_some_and(|v| v.trim().eq_ignore_ascii_case("refresh"))
            })
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Error,
                    "meta refresh reloads or redirects the page automatically (users can't stop it)",
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
        MetaRefresh.check(&dom, src)
    }

    #[test]
    fn refresh_error() {
        assert_eq!(
            check(r#"<head><meta http-equiv="refresh" content="0; url=/x"></head>"#).len(),
            1
        );
    }

    #[test]
    fn http_equiv_case_insensitive() {
        assert_eq!(
            check(r#"<head><meta HTTP-EQUIV="Refresh" content="5"></head>"#).len(),
            1
        );
    }

    #[test]
    fn altri_meta_ok() {
        assert!(check(r#"<head><meta charset="utf-8"><meta http-equiv="content-type" content="text/html"></head>"#).is_empty());
    }
}
