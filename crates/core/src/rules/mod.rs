mod a_no_text;
mod duplicate_id;
mod html_lang;
mod img_alt;
mod img_dimensions;
mod meta_charset;
mod meta_description;
mod meta_viewport;
mod title_present;

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
        // Warn
        Box::new(meta_charset::MetaCharset),
        Box::new(meta_viewport::MetaViewport),
        Box::new(meta_description::MetaDescription),
        Box::new(img_dimensions::ImgDimensions),
        Box::new(a_no_text::ANoText),
    ]
}
