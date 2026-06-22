use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::rule::Rule;
use crate::util::{has_attr, is_a11y_hidden, is_presentational, opening_tag_span};
use tl::VDom;

/// Ogni `<img>` deve avere l'attributo `alt`.
///
/// `alt=""` è valido (immagine decorativa), quindi segnaliamo solo l'attributo
/// del tutto mancante. Saltiamo anche le immagini marcate come decorative in
/// altri modi (`role="presentation"`/`"none"`, `aria-hidden="true"`, `hidden`):
/// segnalarle sarebbe un falso positivo.
pub struct ImgAlt;

impl Rule for ImgAlt {
    fn id(&self) -> &'static str {
        "img-alt"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Error,
            summary: "Every <img> has an alt attribute",
            help: "Add a descriptive alt; use alt=\"\" for purely decorative images.",
            example_bad: r#"<img src="logo.png">"#,
            example_good: r#"<img src="logo.png" alt="Logo Acme">"#,
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/img#alt",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(imgs) = dom.query_selector("img") else {
            return Vec::new();
        };
        imgs.filter_map(|h| h.get(parser)?.as_tag())
            .filter(|tag| !has_attr(tag, "alt"))
            .filter(|tag| !is_presentational(tag) && !is_a11y_hidden(tag))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Error,
                    "<img> is missing an alt attribute",
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
        let mut findings = ImgAlt.check(&dom, src);
        // come fa l'orchestratore: aggancia il sorgente così `snippet()` funziona.
        findings.iter_mut().for_each(|f| f.source = src.into());
        findings
    }

    #[test]
    fn senza_alt_error() {
        let f = check(r#"<img src="a.png">"#);
        assert_eq!(f.len(), 1);
        // lo snippet è il codice reale, non una ricostruzione
        assert_eq!(f[0].snippet(), Some(r#"<img src="a.png">"#));
    }

    #[test]
    fn alt_vuoto_e_valido() {
        // alt="" = immagine decorativa, è corretto.
        assert!(check(r#"<img src="a.png" alt="">"#).is_empty());
    }

    #[test]
    fn decorativa_per_ruolo_o_aria_hidden_non_segnalata() {
        assert!(check(r#"<img src="a.png" role="presentation">"#).is_empty());
        assert!(check(r#"<img src="a.png" role="none">"#).is_empty());
        assert!(check(r#"<img src="a.png" aria-hidden="true">"#).is_empty());
    }
}
