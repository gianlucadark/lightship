use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::{has_attr, opening_tag_span};
use tl::VDom;

/// Ogni `<img>` dovrebbe avere `width` e `height` espliciti: permettono al
/// browser di riservare lo spazio ed evitare il layout shift (CLS).
pub struct ImgDimensions;

impl Rule for ImgDimensions {
    fn id(&self) -> &'static str {
        "img-dimensions"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            summary: "Ogni <img> ha width e height",
            help: "Specifica width e height (o aspect-ratio in CSS) per evitare il layout shift (CLS).",
            example_bad: r#"<img src="hero.png">"#,
            example_good: r#"<img src="hero.png" width="800" height="600">"#,
            docs_url: "https://web.dev/articles/optimize-cls",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(imgs) = dom.query_selector("img") else {
            return Vec::new();
        };
        imgs.filter_map(|h| h.get(parser)?.as_tag())
            .filter(|tag| !has_attr(tag, "width") || !has_attr(tag, "height"))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Warn,
                    "<img> senza width/height (rischio layout shift)",
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
        ImgDimensions.check(&dom, src)
    }

    #[test]
    fn con_dimensioni_ok() {
        assert!(check(r#"<img src="a.png" width="10" height="10">"#).is_empty());
    }

    #[test]
    fn manca_height_warn() {
        assert_eq!(check(r#"<img src="a.png" width="10">"#).len(), 1);
    }
}
