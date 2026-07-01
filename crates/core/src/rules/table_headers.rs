use crate::finding::{Finding, Severity};
use crate::meta::{Category, RuleMeta};
use crate::rule::Rule;
use crate::util::{is_presentational, opening_tag_span};
use tl::{HTMLTag, Parser, VDom};

/// Una tabella di dati (con celle `<td>`) deve avere celle d'intestazione `<th>`:
/// senza, gli screen reader non possono associare ogni cella alla sua colonna/riga
/// e la tabella diventa incomprensibile. Le tabelle di solo layout
/// (`role="presentation"`) sono escluse.
pub struct TableHeaders;

impl Rule for TableHeaders {
    fn id(&self) -> &'static str {
        "table-headers"
    }

    fn meta(&self) -> RuleMeta {
        RuleMeta {
            id: self.id(),
            severity: Severity::Warn,
            category: Category::Accessibility,
            summary: "Data tables have header cells (<th>)",
            help: "Add <th> header cells (with scope) to data tables, or role=\"presentation\" if layout-only.",
            example_bad: "<table><tr><td>Rome</td><td>Italy</td></tr></table>",
            example_good: "<table><tr><th>City</th><th>Country</th></tr><tr><td>Rome</td><td>Italy</td></tr></table>",
            docs_url: "https://developer.mozilla.org/docs/Web/HTML/Element/th",
        }
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding> {
        let parser = dom.parser();
        let Some(tables) = dom.query_selector("table") else {
            return Vec::new();
        };
        tables
            .filter_map(|h| h.get(parser)?.as_tag())
            .filter(|tag| !is_presentational(tag))
            .filter(|tag| is_data_table_without_headers(tag, parser))
            .map(|tag| {
                Finding::new(
                    self.id(),
                    Severity::Warn,
                    "data table has <td> cells but no <th> headers",
                    Some(opening_tag_span(tag, parser, src)),
                )
            })
            .collect()
    }
}

/// Vero se la tabella contiene almeno una `<td>` ma nessuna `<th>`.
fn is_data_table_without_headers(tag: &HTMLTag<'_>, parser: &Parser<'_>) -> bool {
    let html = tag.inner_html(parser);
    contains_tag(&html, b"td") && !contains_tag(&html, b"th")
}

/// Vero se l'HTML contiene un tag di apertura con quel nome (case-insensitive),
/// distinguendo `<th`/`<td` da tag più lunghi come `<thead>`/`<tbody>`: il
/// carattere subito dopo il nome non deve essere alfanumerico.
fn contains_tag(html: &str, name: &[u8]) -> bool {
    let bytes = html.as_bytes();
    let mut i = 0;
    while i + 1 + name.len() < bytes.len() {
        if bytes[i] == b'<' {
            let start = i + 1;
            if bytes[start..start + name.len()].eq_ignore_ascii_case(name)
                && !bytes[start + name.len()].is_ascii_alphanumeric()
            {
                return true;
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Vec<Finding> {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        TableHeaders.check(&dom, src)
    }

    #[test]
    fn con_th_ok() {
        assert!(check("<table><tr><th>City</th></tr><tr><td>Rome</td></tr></table>").is_empty());
    }

    #[test]
    fn solo_td_warn() {
        assert_eq!(
            check("<table><tr><td>Rome</td><td>Italy</td></tr></table>").len(),
            1
        );
    }

    #[test]
    fn presentation_ignorata() {
        assert!(check(r#"<table role="presentation"><tr><td>layout</td></tr></table>"#).is_empty());
    }

    #[test]
    fn thead_non_confuso_con_th() {
        // <thead> senza <th> reali non deve contare come header.
        assert_eq!(
            check("<table><thead><tr><td>x</td></tr></thead></table>").len(),
            1
        );
    }
}
