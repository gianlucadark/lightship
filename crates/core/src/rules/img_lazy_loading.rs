use crate::finding::{Finding, Severity};
use crate::fix::{Edit, Fix};
use crate::meta::{Category, RuleMeta};
use crate::rule::Rule;
use crate::util::{attr, attr_insert_pos, has_attr, opening_tag_span};
use tl::VDom;

/// Suggerisce un attributo `loading` esplicito su ogni `<img>`. È una regola
/// **opinabile** (l'immagine LCP/hero dovrebbe restare `eager`, non `lazy`),
/// quindi è opt-in: fuori dal preset `recommended`, va abilitata con
/// `--preset all` o selezionandola esplicitamente.
pub struct ImgLazyLoading;

impl Rule for ImgLazyLoading {
    fn id(&self) -> &'static str {
        "img-lazy-loading"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Performance,
            summary: "Every <img> declares a loading strategy",
            help: "Set loading=\"lazy\" for below-the-fold images (keep the hero image eager).",
            example_bad: r#"<img src="thumb.png">"#,
            example_good: r#"<img src="thumb.png" loading="lazy">"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/img#loading",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(imgs) = dom.query_selector("img") else {
            return Vec::new();
        };
        imgs.filter_map(|h| h.get(parser)?.as_tag())
            .filter(|tag| !has_loading(tag))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Warn,
                    "<img> has no explicit loading attribute",
                    Some(opening_tag_span(tag, parser, src)),
                )
            })
            .collect()
    }

    fn fixes(&self, dom: &VDom<'_>, src: &str) -> Vec<Fix> {
        let parser = dom.parser();
        let Some(imgs) = dom.query_selector("img") else {
            return Vec::new();
        };
        imgs.filter_map(|h| h.get(parser)?.as_tag())
            .filter(|tag| !has_loading(tag))
            .map(|tag| {
                let span = opening_tag_span(tag, parser, src);
                let at = attr_insert_pos(src, span);
                Fix::new(
                    self.id(),
                    "add loading=\"lazy\" to this <img>",
                    "loading=\"lazy\"",
                    Edit::insert(at, " loading=\"lazy\""),
                )
            })
            .collect()
    }
}

/// Vero se l'immagine ha un `loading` con valore noto (`lazy`/`eager`).
fn has_loading(tag: &tl::HTMLTag<'_>) -> bool {
    has_attr(tag, "loading")
        && attr(tag, "loading").is_some_and(|v| {
            let v = v.trim();
            v.eq_ignore_ascii_case("lazy") || v.eq_ignore_ascii_case("eager")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        ImgLazyLoading.check(&dom, src)
    }

    #[test]
    fn con_loading_ok() {
        assert!(check(r#"<img src="a.png" loading="lazy">"#).is_empty());
        assert!(check(r#"<img src="a.png" loading="eager">"#).is_empty());
    }

    #[test]
    fn senza_loading_warn() {
        assert_eq!(check(r#"<img src="a.png">"#).len(), 1);
    }
}
