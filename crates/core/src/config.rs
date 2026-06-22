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
}

impl Config {
    /// Cerca e carica la config. Priorità: `explicit` (`--config`), poi
    /// `<dir>/lightship.toml`, poi `./lightship.toml`. Se non trovata o non
    /// parsabile, ritorna la config di default segnalando l'errore su stderr.
    pub fn load(dir: &str, explicit: Option<&Path>) -> Config {
        let path = explicit.map(PathBuf::from).or_else(|| {
            [Path::new(dir).join(CONFIG_FILE), PathBuf::from(CONFIG_FILE)]
                .into_iter()
                .find(|p| p.is_file())
        });

        let Some(path) = path else {
            return Config::default();
        };

        match std::fs::read_to_string(&path).map(|s| Self::parse(&s)) {
            Ok(Ok(cfg)) => cfg,
            Ok(Err(e)) => {
                eprintln!("lightship: {} non valido: {e}", path.display());
                Config::default()
            }
            Err(e) => {
                eprintln!("lightship: impossibile leggere {}: {e}", path.display());
                Config::default()
            }
        }
    }

    /// Parsa il contenuto TOML in una `Config`.
    pub fn parse(toml_src: &str) -> Result<Config, String> {
        let raw: Raw = toml::from_str(toml_src).map_err(|e| e.message().to_string())?;

        let mut overrides = HashMap::new();
        for (id, val) in raw.rules {
            let setting = RuleSetting::parse(&val)
                .ok_or_else(|| format!("valore regola sconosciuto per '{id}': '{val}'"))?;
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
            Some(f) => Some(Format::parse(f).ok_or_else(|| format!("formato sconosciuto: '{f}'"))?),
            None => None,
        };

        Ok(Config {
            overrides,
            ignore,
            format,
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
pub const DEFAULT_CONFIG: &str = r#"# Configurazione di Lightship — https://github.com/gianlucadark/lightship
#
# Override gravità o disattivazione delle regole. Valori: "error" | "warn" | "off".
[rules]
# img-alt = "error"
# meta-viewport = "off"

# Percorsi (glob) da escludere dall'analisi.
[ignore]
paths = [
    # "**/404.html",
    # "**/_*.html",
]

[output]
# Formato di default: pretty | compact | json | sarif | github
format = "pretty"
"#;

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
}
