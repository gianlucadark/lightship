use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::{Rule, RuleScope};
use crate::util::{attr, opening_tag_span};
use tl::VDom;

/// Il valore di `<html lang>`, quando presente, deve avere una forma BCP-47
/// plausibile (es. `en`, `it`, `en-US`, `pt-BR`). Errori tipici da segnalare:
/// nomi per esteso (`lang="english"`), separatore sbagliato (`en_US`), spazi.
/// La sola presenza/vuoto è compito di [`html-lang`](super::html_lang); qui
/// valutiamo solo un valore già presente.
pub struct LangValid;

impl Rule for LangValid {
    fn id(&self) -> &'static str {
        "lang-valid"
    }

    fn scope(&self) -> RuleScope {
        RuleScope::Document
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Accessibility,
            summary: "The <html lang> value is a well-formed BCP-47 tag",
            help: "Use a language code like \"en\", \"it\" or \"en-US\" (hyphen, not underscore).",
            example_bad: r#"<html lang="english">"#,
            example_good: r#"<html lang="en-US">"#,
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
            .filter_map(|tag| {
                let value = attr(tag, "lang")?;
                let value = value.trim();
                // Vuoto ⇒ lo gestisce html-lang, non noi.
                if value.is_empty() || is_well_formed(value) {
                    return None;
                }
                Some(Finding::new(
                    self.id(),
                    Severity::Warn,
                    format!("lang=\"{value}\" is not a well-formed language tag"),
                    Some(opening_tag_span(tag, parser, src)),
                ))
            })
            .collect()
    }
}

/// Vero se `tag` ha una forma BCP-47 plausibile: subtag primario di 2–3 lettere,
/// eventuali subtag successivi (regione/script) non vuoti e alfanumerici,
/// separati da trattino.
fn is_well_formed(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let Some(primary) = parts.next() else {
        return false;
    };
    let primary_ok =
        (2..=3).contains(&primary.len()) && primary.bytes().all(|b| b.is_ascii_alphabetic());
    if !primary_ok {
        return false;
    }
    parts.all(|p| !p.is_empty() && p.len() <= 8 && p.bytes().all(|b| b.is_ascii_alphanumeric()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        LangValid.check(&dom, src)
    }

    #[test]
    fn tag_validi_ok() {
        assert!(check(r#"<html lang="en"></html>"#).is_empty());
        assert!(check(r#"<html lang="it"></html>"#).is_empty());
        assert!(check(r#"<html lang="en-US"></html>"#).is_empty());
        assert!(check(r#"<html lang="pt-BR"></html>"#).is_empty());
    }

    #[test]
    fn nome_per_esteso_warn() {
        assert_eq!(check(r#"<html lang="english"></html>"#).len(), 1);
    }

    #[test]
    fn underscore_warn() {
        assert_eq!(check(r#"<html lang="en_US"></html>"#).len(), 1);
    }

    #[test]
    fn vuoto_non_nostro() {
        // La presenza/vuoto è compito di html-lang.
        assert!(check(r#"<html lang=""></html>"#).is_empty());
        assert!(check("<html></html>").is_empty());
    }
}
