use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::{attr, has_attr, opening_tag_span};
use tl::{HTMLTag, Parser, VDom};

/// Alcuni tag della `<head>` devono comparire **una sola volta**: un secondo
/// `<meta charset>`, `<meta name="viewport">`, `<meta name="description">` o
/// `<title>` è ignorato (o peggio, sceglie il browser) e confonde SEO e social
/// preview. Segnaliamo le occorrenze oltre la prima.
pub struct DuplicateMeta;

impl Rule for DuplicateMeta {
    fn id(&self) -> &'static str {
        "duplicate-meta"
    }

    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Seo,
            summary: "Single-instance head tags are not duplicated",
            help: "Keep one <title> and one meta charset/viewport/description; remove the extras.",
            example_bad: r#"<title>A</title><title>B</title>"#,
            example_good: r#"<title>A</title>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/meta",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let mut out = Vec::new();

        // <meta charset> e <meta name="..."> unici.
        if let Some(metas) = dom.query_selector("meta") {
            let tags: Vec<&HTMLTag> = metas.filter_map(|h| h.get(parser)?.as_tag()).collect();
            self.flag_extras(
                &mut out,
                parser,
                src,
                tags.iter().copied().filter(|t| has_attr(t, "charset")),
                "<meta charset>",
            );
            for name in ["viewport", "description"] {
                self.flag_extras(
                    &mut out,
                    parser,
                    src,
                    tags.iter().copied().filter(|t| meta_named(t, name)),
                    &format!("<meta name=\"{name}\">"),
                );
            }
        }

        // <title> unico.
        if let Some(titles) = dom.query_selector("title") {
            let tags = titles.filter_map(|h| h.get(parser)?.as_tag());
            self.flag_extras(&mut out, parser, src, tags, "<title>");
        }

        out
    }
}

impl DuplicateMeta {
    /// Aggiunge un finding per ogni occorrenza dopo la prima.
    fn flag_extras<'a>(
        &self,
        out: &mut Vec<Finding>,
        parser: &Parser<'_>,
        src: &str,
        tags: impl Iterator<Item = &'a HTMLTag<'a>>,
        what: &str,
    ) {
        for tag in tags.skip(1) {
            out.push(Finding::new(
                self.id(),
                Severity::Warn,
                format!("duplicate {what} in the document"),
                Some(opening_tag_span(tag, parser, src)),
            ));
        }
    }
}

/// Vero se il `<meta>` ha `name` uguale (case-insensitive) a `name`.
fn meta_named(tag: &HTMLTag<'_>, name: &str) -> bool {
    attr(tag, "name").is_some_and(|v| v.trim().eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        DuplicateMeta.check(&dom, src)
    }

    #[test]
    fn singoli_ok() {
        assert!(
            check(
                r#"<head><meta charset="utf-8"><meta name="viewport" content="x"><title>A</title></head>"#
            )
            .is_empty()
        );
    }

    #[test]
    fn title_duplicato_warn() {
        assert_eq!(
            check("<head><title>A</title><title>B</title></head>").len(),
            1
        );
    }

    #[test]
    fn charset_e_viewport_duplicati_warn() {
        let f = check(
            r#"<head><meta charset="utf-8"><meta charset="latin1"><meta name="viewport" content="a"><meta name="viewport" content="b"></head>"#,
        );
        assert_eq!(f.len(), 2);
    }
}
