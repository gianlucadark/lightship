use crate::finding::Severity;
use crate::report::Format;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Nome del file di configurazione cercato nella cartella analizzata / cwd.
pub const CONFIG_FILE: &str = "lightship.toml";

/// Impostazione di una regola nella config: override di gravità o disattivazione.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSetting {
    Error,
    Warn,
    Off,
}

impl RuleSetting {
    fn parse(s: &str) -> Option<RuleSetting> {
        match s.trim().to_ascii_lowercase().as_str() {
            "error" | "err" => Some(RuleSetting::Error),
            "warn" | "warning" => Some(RuleSetting::Warn),
            "off" | "false" | "disabled" => Some(RuleSetting::Off),
            _ => None,
        }
    }
}

/// Configurazione effettiva applicata da `analyze`. Costruita da `lightship.toml`
/// (assente ⇒ default che non cambia nulla).
#[derive(Debug, Default)]
pub struct Config {
    overrides: HashMap<String, RuleSetting>,
    ignore: Option<GlobSet>,
    /// Formato di output di default (sovrascrivibile da `--format`).
    pub format: Option<Format>,
    /// Esegue le regole "di documento" anche sui frammenti/partial.
    pub include_fragments: bool,
    /// Dimensione massima (byte) di un file analizzato; oltre viene saltato.
    pub max_file_bytes: Option<usize>,
    /// Preset di regole di default (`recommended`/`all`/categoria).
    pub preset: Option<String>,
    /// Soglia CI di warning di default (sovrascrivibile da `--max-warnings`).
    pub ci_max_warnings: Option<usize>,
    /// Se `true`, in CI qualunque warning fa fallire la build.
    pub ci_error_on_warnings: bool,
}

impl Config {
    /// Cerca e carica la config. Priorità: `explicit` (`--config`), poi
    /// `<dir>/lightship.toml`, poi `./lightship.toml`. Se non trovata o non
    /// parsabile, ritorna la config di default segnalando l'errore su stderr.
    pub fn load(dir: &str, explicit: Option<&Path>) -> Config {
        Self::try_load(dir, explicit).unwrap_or_else(|e| {
            eprintln!("lightship: {e}");
            Config::default()
        })
    }

    /// Come [`load`](Self::load) ma **fallisce** se una config esiste e non è
    /// leggibile/parsabile (o se il `--config` esplicito non esiste), invece di
    /// degradare in silenzio al default: una config invalida ignorerebbe soglie
    /// CI e regole disattivate. La CLI mappa l'errore sull'exit code 2.
    pub fn try_load(dir: &str, explicit: Option<&Path>) -> Result<Config, String> {
        let path = explicit.map(PathBuf::from).or_else(|| {
            [Path::new(dir).join(CONFIG_FILE), PathBuf::from(CONFIG_FILE)]
                .into_iter()
                .find(|p| p.is_file())
        });

        let Some(path) = path else {
            return Ok(Config::default());
        };

        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        Self::parse(&raw).map_err(|e| format!("invalid {}: {e}", path.display()))
    }

    /// Parsa il contenuto TOML in una `Config`.
    pub fn parse(toml_src: &str) -> Result<Config, String> {
        let raw: Raw = toml::from_str(toml_src).map_err(|e| e.message().to_string())?;

        let mut overrides = HashMap::new();
        for (id, val) in raw.rules {
            let setting = RuleSetting::parse(&val)
                .ok_or_else(|| format!("unknown rule value for '{id}': '{val}'"))?;
            overrides.insert(id, setting);
        }

        let ignore = if raw.ignore.paths.is_empty() {
            None
        } else {
            let mut builder = GlobSetBuilder::new();
            for pat in &raw.ignore.paths {
                let glob = Glob::new(pat).map_err(|e| format!("glob '{pat}': {e}"))?;
                builder.add(glob);
            }
            Some(builder.build().map_err(|e| e.to_string())?)
        };

        let format = match raw.output.format.as_deref() {
            Some(f) => Some(Format::parse(f).ok_or_else(|| format!("unknown format: '{f}'"))?),
            None => None,
        };

        Ok(Config {
            overrides,
            ignore,
            format,
            include_fragments: raw.analyze.include_fragments,
            max_file_bytes: raw.analyze.max_file_bytes,
            preset: raw.analyze.preset,
            ci_max_warnings: raw.ci.max_warnings,
            ci_error_on_warnings: raw.ci.error_on_warnings,
        })
    }

    /// `true` se la regola è disattivata via config.
    pub fn is_rule_off(&self, id: &str) -> bool {
        self.overrides.get(id) == Some(&RuleSetting::Off)
    }

    /// La gravità effettiva di un finding: override della config se presente,
    /// altrimenti quella di default della regola.
    pub fn severity_for(&self, id: &str, default: Severity) -> Severity {
        match self.overrides.get(id) {
            Some(RuleSetting::Error) => Severity::Error,
            Some(RuleSetting::Warn) => Severity::Warn,
            _ => default,
        }
    }

    /// `true` se il percorso (relativo alla cartella analizzata) è da ignorare.
    pub fn is_ignored(&self, path: &Path) -> bool {
        // globset ragiona con separatori `/`: normalizziamo i path di Windows.
        let s = path.to_string_lossy().replace('\\', "/");
        self.ignore.as_ref().is_some_and(|g| g.is_match(&s))
    }
}

#[derive(Deserialize, Default)]
struct Raw {
    #[serde(default)]
    rules: HashMap<String, String>,
    #[serde(default)]
    ignore: RawIgnore,
    #[serde(default)]
    output: RawOutput,
    #[serde(default)]
    analyze: RawAnalyze,
    #[serde(default)]
    ci: RawCi,
}

#[derive(Deserialize, Default)]
struct RawCi {
    #[serde(default)]
    max_warnings: Option<usize>,
    #[serde(default)]
    error_on_warnings: bool,
}

#[derive(Deserialize, Default)]
struct RawAnalyze {
    #[serde(default)]
    include_fragments: bool,
    #[serde(default)]
    max_file_bytes: Option<usize>,
    #[serde(default)]
    preset: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawIgnore {
    #[serde(default)]
    paths: Vec<String>,
}

#[derive(Deserialize, Default)]
struct RawOutput {
    #[serde(default)]
    format: Option<String>,
}

/// Il contenuto del `lightship.toml` di default scritto da `lightship init`.
pub const DEFAULT_CONFIG: &str = r#"# Lightship configuration — https://github.com/gianlucadark/lightship
#
# Override rule severity or turn rules off. Values: "error" | "warn" | "off".
[rules]
# img-alt = "error"
# meta-viewport = "off"

# Glob paths to exclude from the analysis.
[ignore]
paths = [
    # "**/404.html",
    # "**/_*.html",
]

[analyze]
# Document-level rules (title, charset, viewport, single-h1...) are skipped on
# HTML fragments/partials without <html>/<head>. Set to true to run them anyway.
# include_fragments = false
# Skip files larger than this many bytes (default 8 MiB).
# max_file_bytes = 8388608

[output]
# Default output format: pretty | compact | json | sarif | github
format = "pretty"
"#;

/// Workflow CI di default scritto da `lightship ci`, già pronto per `<dir>`.
/// Vedi [`ci_workflow`] per inserire la cartella di build rilevata.
pub const DEFAULT_CI_WORKFLOW: &str = r#"# GitHub Actions workflow generated by `lightship ci`.
# Builds the project and fails the job if Lightship finds errors.
name: lightship

on:
  push:
    branches: [main]
  pull_request:

jobs:
  lint-html:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - run: npm ci
      - run: npm run build
      - run: npx lightship {DIR}
"#;

/// Costruisce il contenuto del workflow CI puntandolo a `dir` (la cartella di
/// build). Sostituisce il segnaposto `{DIR}` in [`DEFAULT_CI_WORKFLOW`].
pub fn ci_workflow(dir: &str) -> String {
    DEFAULT_CI_WORKFLOW.replace("{DIR}", dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_e_off() {
        let cfg = Config::parse("[rules]\nimg-alt = \"warn\"\nhtml-lang = \"off\"\n").unwrap();
        assert_eq!(cfg.severity_for("img-alt", Severity::Error), Severity::Warn);
        assert!(cfg.is_rule_off("html-lang"));
        // regola non citata: gravità di default invariata
        assert_eq!(
            cfg.severity_for("title-present", Severity::Error),
            Severity::Error
        );
    }

    #[test]
    fn ignore_glob() {
        let cfg = Config::parse("[ignore]\npaths = [\"**/404.html\"]\n").unwrap();
        assert!(cfg.is_ignored(Path::new("dist/404.html")));
        assert!(!cfg.is_ignored(Path::new("dist/index.html")));
    }

    #[test]
    fn default_config_e_parsabile() {
        assert!(Config::parse(DEFAULT_CONFIG).is_ok());
    }

    #[test]
    fn valore_sconosciuto_errore() {
        assert!(Config::parse("[rules]\nimg-alt = \"boh\"\n").is_err());
    }

    #[test]
    fn sezione_analyze() {
        let cfg =
            Config::parse("[analyze]\ninclude_fragments = true\nmax_file_bytes = 1024\n").unwrap();
        assert!(cfg.include_fragments);
        assert_eq!(cfg.max_file_bytes, Some(1024));
        // assente ⇒ default
        let cfg = Config::parse("").unwrap();
        assert!(!cfg.include_fragments);
        assert_eq!(cfg.max_file_bytes, None);
    }
}
