use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::Rule;
use crate::util::opening_tag_span;
use tl::VDom;

/// Tag di presentazione obsoleti in HTML5: il rendering non è più garantito e
/// vanno sostituiti con CSS o con l'elemento semantico equivalente.
pub struct DeprecatedTag;

/// Elementi obsoleti segnalati (con il rimpiazzo suggerito nel messaggio).
const DEPRECATED: &[(&str, &str)] = &[
    ("center", "CSS text-align / margin auto"),
    ("font", "CSS font-* properties"),
    ("marquee", "CSS animations"),
    ("big", "CSS font-size"),
    ("blink", "CSS animations"),
    ("acronym", "<abbr>"),
];

impl Rule for DeprecatedTag {
    fn id(&self) -> &'static str {
        "deprecated-tag"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Correctness,
            summary: "no obsolete HTML tags (<center>, <font>, <marquee>…)",
            help: "Replace obsolete presentational tags with CSS or the semantic equivalent (<abbr> for <acronym>).",
            example_bad: "<center><font size=\"5\">Title</font></center>",
            example_good: r#"<h1 style="text-align:center">Title</h1>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element#obsolete_and_deprecated_elements",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let mut findings = Vec::new();
        for (name, replacement) in DEPRECATED {
            let Some(els) = dom.query_selector(name) else {
                continue;
            };
            for tag in els.filter_map(|h| h.get(parser)?.as_tag()) {
                findings.push(Finding::new(
                    self.id(),
                    Severity::Warn,
                    format!("<{name}> is obsolete in HTML5 (use {replacement})"),
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
        DeprecatedTag.check(&dom, src)
    }

    #[test]
    fn center_e_font_warn() {
        assert_eq!(check("<center><font>x</font></center>").len(), 2);
    }

    #[test]
    fn marquee_warn() {
        assert_eq!(check("<marquee>news</marquee>").len(), 1);
    }

    #[test]
    fn tag_moderni_ok() {
        assert!(check("<div><strong>x</strong><abbr>HTML</abbr></div>").is_empty());
    }
}
