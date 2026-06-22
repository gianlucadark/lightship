//! Rendering dei risultati: dashboard "pretty", compatto e formati machine
//! (JSON, SARIF, GitHub). Tutti i renderer producono una `String`; i colori
//! ANSI vengono emessi sempre e poi rimossi a valle da `anstream` quando serve.

mod commands;
mod compact;
mod github;
mod json;
mod pretty;
mod sarif;
mod snippet;
mod summary;

pub use commands::{explain, rules_table};

use crate::Analysis;
use crate::meta::RuleMeta;

/// Formato di output scelto per `analyze`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Dashboard a colori (default).
    Pretty,
    /// Una riga per finding, denso (utile in log/CI).
    Compact,
    /// JSON strutturato.
    Json,
    /// SARIF 2.1.0 (GitHub code scanning).
    Sarif,
    /// Workflow command di GitHub Actions (annotazioni inline).
    Github,
}

impl Format {
    /// Parsa il nome del formato (case-insensitive).
    pub fn parse(s: &str) -> Option<Format> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pretty" | "dashboard" => Some(Format::Pretty),
            "compact" => Some(Format::Compact),
            "json" => Some(Format::Json),
            "sarif" => Some(Format::Sarif),
            "github" | "gh" => Some(Format::Github),
            _ => None,
        }
    }
}

/// Scelta del colore per lo stream di output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    /// Colore se l'output è un terminale (rispetta `NO_COLOR`/`CLICOLOR`).
    #[default]
    Auto,
    Always,
    Never,
}

impl From<Color> for anstream::ColorChoice {
    fn from(c: Color) -> Self {
        match c {
            Color::Auto => anstream::ColorChoice::Auto,
            Color::Always => anstream::ColorChoice::Always,
            Color::Never => anstream::ColorChoice::Never,
        }
    }
}

/// Opzioni passate ai renderer testuali.
pub struct RenderOpts {
    pub suggestions: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub dir: String,
}

/// Renderizza l'analisi nel formato richiesto.
pub fn render(format: Format, analysis: &Analysis, opts: &RenderOpts) -> String {
    match format {
        Format::Pretty => pretty::render(analysis, opts),
        Format::Compact => compact::render(analysis, opts),
        Format::Json => json::render(analysis),
        Format::Sarif => sarif::render(analysis),
        Format::Github => github::render(analysis),
    }
}

/// Cerca i metadati di una regola in un registro già materializzato.
pub(crate) fn find_meta<'a>(metas: &'a [RuleMeta], id: &str) -> Option<&'a RuleMeta> {
    metas.iter().find(|m| m.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::{Finding, Severity};
    use std::sync::Arc;
    use std::time::Duration;

    fn sample() -> Analysis {
        let mut f = Finding::new("img-alt", Severity::Error, "<img> senza alt", Some((0, 5)));
        f.attach("dist/index.html".into(), Arc::from("<img>\n"));
        Analysis {
            pages: 1,
            findings: vec![f],
            elapsed: Duration::from_millis(3),
        }
    }

    #[test]
    fn json_e_valido_e_completo() {
        let v: serde_json::Value = serde_json::from_str(&json::render(&sample())).unwrap();
        assert_eq!(v["summary"]["errors"], 1);
        assert_eq!(v["findings"][0]["rule"], "img-alt");
        assert_eq!(v["findings"][0]["line"], 1);
        assert!(v["findings"][0]["help"].is_string());
    }

    #[test]
    fn sarif_e_valido() {
        let v: serde_json::Value = serde_json::from_str(&sarif::render(&sample())).unwrap();
        assert_eq!(v["version"], "2.1.0");
        assert_eq!(v["runs"][0]["results"][0]["ruleId"], "img-alt");
        assert_eq!(v["runs"][0]["results"][0]["level"], "error");
    }

    #[test]
    fn format_parse() {
        assert_eq!(Format::parse("JSON"), Some(Format::Json));
        assert_eq!(Format::parse("gh"), Some(Format::Github));
        assert_eq!(Format::parse("boh"), None);
    }

    #[test]
    fn pretty_report_has_a_clear_visual_hierarchy() {
        let report = pretty::render(
            &sample(),
            &RenderOpts {
                suggestions: true,
                verbose: false,
                quiet: false,
                dir: "dist".to_string(),
            },
        );
        assert!(report.contains("CHECK FAILED"));
        assert!(report.contains("dist/index.html"));
        assert!(report.contains("ERROR"));
        assert!(report.contains("Fix"));
        assert!(report.contains("Results"));
    }
}
