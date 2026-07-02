use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::{attr, attr_non_empty};
use tl::VDom;

/// Senza i tag Open Graph di base la condivisione su social e chat mostra
/// un'anteprima anonima (titolo grezzo, niente immagine). Regola **opt-in**
/// (fuori dal preset `recommended`): non tutti i siti puntano alla condivisione.
pub struct OgBasic;

/// I tag Open Graph considerati "di base" per un'anteprima decente.
const REQUIRED: &[&str] = &["og:title", "og:description", "og:image"];

impl Rule for OgBasic {
    fn id(&self) -> &'static str {
        "og-basic"
    }

    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Seo,
            summary: "basic Open Graph tags are present (og:title, og:description, og:image)",
            help: "Add <meta property=\"og:title\">, og:description and og:image so shared links get a rich preview.",
            example_bad: "<head><title>Post</title></head>",
            example_good: r#"<meta property="og:title" content="Post title">"#,
            docs_url: "https://ogp.me/",
        }
    }

    fn check(&self, dom: &VDom<'_>, _src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        // Raccoglie le property `og:*` presenti con un content non vuoto
        // (accettiamo anche `name=` al posto di `property=`: capita spesso).
        let mut present: Vec<String> = Vec::new();
        if let Some(metas) = dom.query_selector("meta") {
            for tag in metas.filter_map(|h| h.get(parser)?.as_tag()) {
                let prop = attr(tag, "property").or_else(|| attr(tag, "name"));
                if let Some(p) = prop
                    && attr_non_empty(tag, "content")
                {
                    present.push(p.trim().to_ascii_lowercase());
                }
            }
        }
        let missing: Vec<&str> = REQUIRED
            .iter()
            .copied()
            .filter(|req| !present.iter().any(|p| p == req))
            .collect();
        if missing.is_empty() {
            return Vec::new();
        }
        vec![Finding::new(
            self.id(),
            Severity::Warn,
            format!("missing Open Graph tags: {}", missing.join(", ")),
            None,
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        OgBasic.check(&dom, src)
    }

    #[test]
    fn tutti_presenti_ok() {
        let src = r#"<head>
            <meta property="og:title" content="T">
            <meta property="og:description" content="D">
            <meta property="og:image" content="/i.png">
        </head>"#;
        assert!(check(src).is_empty());
    }

    #[test]
    fn name_al_posto_di_property_ok() {
        let src = r#"<head>
            <meta name="og:title" content="T">
            <meta name="og:description" content="D">
            <meta name="og:image" content="/i.png">
        </head>"#;
        assert!(check(src).is_empty());
    }

    #[test]
    fn mancanti_elencati_nel_messaggio() {
        let src = r#"<head><meta property="og:title" content="T"></head>"#;
        let findings = check(src);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("og:description"));
        assert!(findings[0].message.contains("og:image"));
        assert!(!findings[0].message.contains("og:title,"));
    }

    #[test]
    fn content_vuoto_conta_come_mancante() {
        let src = r#"<head>
            <meta property="og:title" content="">
            <meta property="og:description" content="D">
            <meta property="og:image" content="/i.png">
        </head>"#;
        assert_eq!(check(src).len(), 1);
    }
}
