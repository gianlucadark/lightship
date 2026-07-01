use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::Rule;
use crate::util::{attr_non_empty, is_a11y_hidden, opening_tag_span};
use tl::VDom;

/// Ogni `<iframe>` deve avere un `title` (o un `aria-label`) non vuoto: gli screen
/// reader lo annunciano per spiegare cosa contiene il frame (mappa, video, …).
pub struct IframeTitle;

impl Rule for IframeTitle {
    fn id(&self) -> &'static str {
        "iframe-title"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Accessibility,
            summary: "Every <iframe> has a non-empty title",
            help: "Add a title describing the frame, e.g. <iframe title=\"Location map\">.",
            example_bad: r#"<iframe src="map.html"></iframe>"#,
            example_good: r#"<iframe src="map.html" title="Location map"></iframe>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/iframe#accessibility",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(frames) = dom.query_selector("iframe") else {
            return Vec::new();
        };
        frames
            .filter_map(|h| h.get(parser)?.as_tag())
            .filter(|tag| !is_a11y_hidden(tag))
            .filter(|tag| !attr_non_empty(tag, "title") && !attr_non_empty(tag, "aria-label"))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Warn,
                    "<iframe> is missing a title",
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
        IframeTitle.check(&dom, src)
    }

    #[test]
    fn con_title_ok() {
        assert!(check(r#"<iframe src="m.html" title="Mappa"></iframe>"#).is_empty());
    }

    #[test]
    fn senza_title_warn() {
        assert_eq!(check(r#"<iframe src="m.html"></iframe>"#).len(), 1);
    }

    #[test]
    fn aria_hidden_non_segnalato() {
        assert!(check(r#"<iframe src="m.html" aria-hidden="true"></iframe>"#).is_empty());
    }
}
