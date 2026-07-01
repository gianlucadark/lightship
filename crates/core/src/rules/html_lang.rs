use crate::finding::{Finding, Severity};
use crate::fix::{Edit, Fix};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::{attr, attr_insert_pos, has_attr, opening_tag_span};
use tl::VDom;

/// Il tag `<html>` deve avere un attributo `lang` non vuoto.
pub struct HtmlLang;

impl Rule for HtmlLang {
    fn id(&self) -> &'static str {
        "html-lang"
    }

    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Error,
            category: Category::Accessibility,
            summary: "The <html> tag has a non-empty lang attribute",
            help: "Declare the page language on <html>, e.g. <html lang=\"en\">.",
            example_bad: "<html>",
            example_good: r#"<html lang="en">"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Global_attributes/lang",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(htmls) = dom.query_selector("html") else {
            return Vec::new();
        };
        htmls
            .filter_map(|h| h.get(parser)?.as_tag())
            .filter(|tag| attr(tag, "lang").is_none_or(|v| v.trim().is_empty()))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Error,
                    "<html> is missing a lang attribute",
                    Some(opening_tag_span(tag, parser, src)),
                )
            })
            .collect()
    }

    fn fixes(&self, dom: &VDom<'_>, src: &str) -> Vec<Fix> {
        let parser = dom.parser();
        let Some(htmls) = dom.query_selector("html") else {
            return Vec::new();
        };
        htmls
            .filter_map(|h| h.get(parser)?.as_tag())
            // Solo l'attributo del tutto assente ha un fix non ambiguo (inseriamo
            // `en` come default); un valore presente ma vuoto va corretto a mano.
            .filter(|tag| !has_attr(tag, "lang"))
            .map(|tag| {
                let span = opening_tag_span(tag, parser, src);
                let at = attr_insert_pos(src, span);
                Fix::new(
                    self.id(),
                    "add lang=\"en\" to <html> (change if the page isn't English)",
                    "lang=\"en\"",
                    Edit::insert(at, " lang=\"en\""),
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
        let mut findings = HtmlLang.check(&dom, src);
        findings.iter_mut().for_each(|f| f.source = src.into());
        findings
    }

    #[test]
    fn con_lang_ok() {
        assert!(check(r#"<html lang="it"></html>"#).is_empty());
    }

    #[test]
    fn senza_lang_error() {
        let f = check("<html></html>");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].snippet(), Some("<html>"));
    }

    #[test]
    fn lang_vuoto_error() {
        assert_eq!(check(r#"<html lang="  "></html>"#).len(), 1);
    }
}
