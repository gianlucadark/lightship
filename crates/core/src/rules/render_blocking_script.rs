use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::{attr, has_attr, opening_tag_span};
use tl::{HTMLTag, VDom};

/// Uno `<script src>` classico dentro `<head>` senza `async`/`defer` blocca il
/// parsing e il rendering della pagina. Suggeriamo `defer` (o `type="module"`,
/// già differito) per non ritardare il primo render.
pub struct RenderBlockingScript;

impl Rule for RenderBlockingScript {
    fn id(&self) -> &'static str {
        "render-blocking-script"
    }

    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Performance,
            summary: "No render-blocking <script> in <head>",
            help: "Add defer (or type=\"module\") to head scripts, or move them before </body>.",
            example_bad: r#"<head><script src="app.js"></script></head>"#,
            example_good: r#"<head><script src="app.js" defer></script></head>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/script#defer",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(head) = dom
            .query_selector("head")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser)?.as_tag())
        else {
            return Vec::new();
        };

        head.children()
            .all(parser)
            .iter()
            .filter_map(|n| n.as_tag())
            .filter(|t| t.name().as_bytes().eq_ignore_ascii_case(b"script"))
            .filter(|t| is_render_blocking(t))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Warn,
                    "render-blocking <script> in <head> (add defer or type=\"module\")",
                    Some(opening_tag_span(tag, parser, src)),
                )
            })
            .collect()
    }
}

/// Vero per uno script *classico* con sorgente esterna che blocca il rendering:
/// ha `src`, non è `async`/`defer`, e non è un `type` non eseguibile o `module`.
fn is_render_blocking(tag: &HTMLTag<'_>) -> bool {
    if !has_attr(tag, "src") || has_attr(tag, "async") || has_attr(tag, "defer") {
        return false;
    }
    match attr(tag, "type") {
        None => true,
        Some(t) => {
            let t = t.trim().to_ascii_lowercase();
            t.is_empty() || t == "text/javascript" || t == "application/javascript"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        RenderBlockingScript.check(&dom, src)
    }

    #[test]
    fn defer_ok() {
        assert!(check(r#"<head><script src="a.js" defer></script></head>"#).is_empty());
    }

    #[test]
    fn module_ok() {
        assert!(check(r#"<head><script src="a.js" type="module"></script></head>"#).is_empty());
    }

    #[test]
    fn classico_in_head_warn() {
        assert_eq!(
            check(r#"<head><script src="a.js"></script></head>"#).len(),
            1
        );
    }

    #[test]
    fn script_nel_body_ignorato() {
        assert!(check(r#"<head></head><body><script src="a.js"></script></body>"#).is_empty());
    }

    #[test]
    fn inline_ignorato() {
        // Senza src non scarica nulla: fuori scope di questa regola.
        assert!(check(r#"<head><script>const x=1;</script></head>"#).is_empty());
    }
}
