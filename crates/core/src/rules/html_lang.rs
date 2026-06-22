use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::{attr, opening_tag_span};
use tl::VDom;

/// Il tag `<html>` deve avere un attributo `lang` non vuoto.
pub struct HtmlLang;

impl Rule for HtmlLang {
    fn id(&self) -> &'static str {
        "html-lang"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Error,
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
