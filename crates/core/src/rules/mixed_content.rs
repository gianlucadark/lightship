use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::Rule;
use crate::util::{attr, opening_tag_span};
use tl::VDom;

/// Risorse caricate via `http://` su una pagina servita in HTTPS sono *mixed
/// content*: i browser le bloccano (script/iframe) o le declassano (immagini),
/// rompendo la pagina in produzione. Segnaliamo i sotto-caricamenti espliciti
/// in `http://`; i link di navigazione (`<a>`) non sono mixed content e non
/// vengono toccati.
pub struct MixedContent;

/// Coppie (selettore, attributo con l'URL) da controllare.
const TARGETS: &[(&str, &str)] = &[
    ("script", "src"),
    ("link", "href"),
    ("img", "src"),
    ("iframe", "src"),
];

impl Rule for MixedContent {
    fn id(&self) -> &'static str {
        "mixed-content"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Security,
            summary: "resources are not loaded over insecure http://",
            help: "Load the resource over https:// (or use a relative / protocol-relative URL).",
            example_bad: r#"<script src="http://cdn.example.com/app.js"></script>"#,
            example_good: r#"<script src="https://cdn.example.com/app.js"></script>"#,
            docs_url: "https://developer.mozilla.org/docs/Web/Security/Mixed_content",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let mut findings = Vec::new();
        for (selector, url_attr) in TARGETS {
            let Some(els) = dom.query_selector(selector) else {
                continue;
            };
            for tag in els.filter_map(|h| h.get(parser)?.as_tag()) {
                let insecure = attr(tag, url_attr).is_some_and(|v| {
                    let v = v.trim();
                    v.len() >= 7 && v[..7].eq_ignore_ascii_case("http://")
                });
                if insecure {
                    findings.push(Finding::new(
                        self.id(),
                        Severity::Warn,
                        format!(
                            "<{selector}> loads over insecure http:// (blocked or downgraded on https pages)"
                        ),
                        Some(opening_tag_span(tag, parser, src)),
                    ));
                }
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
        MixedContent.check(&dom, src)
    }

    #[test]
    fn script_http_warn() {
        assert_eq!(check(r#"<script src="http://cdn.x.com/a.js"></script>"#).len(), 1);
    }

    #[test]
    fn https_e_relativi_ok() {
        assert!(check(r#"<script src="https://cdn.x.com/a.js"></script>"#).is_empty());
        assert!(check(r#"<img src="/img/a.png">"#).is_empty());
        assert!(check(r#"<link rel="stylesheet" href="//cdn.x.com/a.css">"#).is_empty());
    }

    #[test]
    fn anchor_di_navigazione_ignorato() {
        assert!(check(r#"<a href="http://example.com">x</a>"#).is_empty());
    }

    #[test]
    fn tutti_i_target_controllati() {
        let src = r#"<link href="http://x.com/a.css"><img src="http://x.com/a.png"><iframe src="http://x.com"></iframe>"#;
        assert_eq!(check(src).len(), 3);
    }
}
