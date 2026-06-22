mod a_no_text;
mod button_name;
mod canonical_link;
mod duplicate_id;
mod html_lang;
mod iframe_title;
mod img_alt;
mod img_dimensions;
mod label_control;
mod link_target_blank;
mod meta_charset;
mod meta_description;
mod meta_description_length;
mod meta_viewport;
mod render_blocking_script;
mod single_h1;
mod title_length;
mod title_present;

use crate::meta::RuleMeta;
use crate::rule::Rule;

/// L'insieme di tutte le regole attive. Ogni regola vive nel suo modulo.
///
/// Convenzione: regole `Error` = bug/accessibilità che rompono la pagina;
/// regole `Warn` = best practice di SEO/performance/UX.
pub fn all() -> Vec<Box<dyn Rule>> {
    vec![
        // Error
        Box::new(img_alt::ImgAlt),
        Box::new(html_lang::HtmlLang),
        Box::new(title_present::TitlePresent),
        Box::new(duplicate_id::DuplicateId),
        // Warn — accessibilità
        Box::new(a_no_text::ANoText),
        Box::new(button_name::ButtonName),
        Box::new(label_control::LabelControl),
        Box::new(iframe_title::IframeTitle),
        // Warn — SEO
        Box::new(meta_charset::MetaCharset),
        Box::new(meta_viewport::MetaViewport),
        Box::new(meta_description::MetaDescription),
        Box::new(meta_description_length::MetaDescriptionLength),
        Box::new(title_length::TitleLength),
        Box::new(single_h1::SingleH1),
        Box::new(canonical_link::CanonicalLink),
        // Warn — performance / sicurezza
        Box::new(img_dimensions::ImgDimensions),
        Box::new(render_blocking_script::RenderBlockingScript),
        Box::new(link_target_blank::LinkTargetBlank),
    ]
}

/// I metadati di tutte le regole, nell'ordine di [`all`]. Fonte per i comandi
/// `rules`/`explain`, i suggerimenti del report e la lista regole SARIF.
pub fn registry() -> Vec<RuleMeta> {
    all().iter().map(|r| r.meta()).collect()
}

/// I metadati di una singola regola per `id`, se esiste.
pub fn meta(id: &str) -> Option<RuleMeta> {
    all().iter().find(|r| r.id() == id).map(|r| r.meta())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_coerente_con_id_e_non_vuota() {
        for rule in all() {
            let m = rule.meta();
            assert_eq!(m.id, rule.id(), "meta.id deve combaciare con id()");
            assert!(!m.summary.is_empty(), "{}: summary vuota", m.id);
            assert!(!m.help.is_empty(), "{}: help vuoto", m.id);
            assert!(m.docs_url.starts_with("https://"), "{}: docs_url", m.id);
        }
    }

    #[test]
    fn registry_e_lookup() {
        assert_eq!(registry().len(), all().len());
        assert!(meta("img-alt").is_some());
        assert!(meta("inesistente").is_none());
    }
}
