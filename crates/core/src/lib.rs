mod config;
mod finding;
mod meta;
mod report;
mod rule;
mod rules;
mod util;

pub use config::{CONFIG_FILE, Config, DEFAULT_CONFIG};
pub use finding::{Finding, Severity};
pub use meta::RuleMeta;
pub use report::{Color, Format, RenderOpts};
pub use rule::Rule;

use ignore::WalkBuilder;
use rayon::prelude::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Risultato dell'analisi di una cartella.
pub struct Analysis {
    pub findings: Vec<Finding>,
    pub pages: usize,
    pub elapsed: Duration,
}

impl Analysis {
    /// Numero di finding di gravità `Error`.
    pub fn errors(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count()
    }

    /// Numero di finding di gravità `Warn`.
    pub fn warnings(&self) -> usize {
        self.findings.len() - self.errors()
    }
}

/// Opzioni di esecuzione della CLI/libreria.
#[derive(Debug, Clone)]
pub struct Options {
    /// Stampa solo il riepilogo, non i singoli finding.
    pub quiet: bool,
    /// Dettagli extra (al momento: nessun troncamento degli snippet lunghi).
    pub verbose: bool,
    /// Mostra la riga 💡 con il suggerimento di fix.
    pub suggestions: bool,
    /// Formato di output; `None` ⇒ usa quello della config, poi `Pretty`.
    pub format: Option<Format>,
    /// Scelta colore per lo stream di output.
    pub color: Color,
    /// Se impostato, fa fallire la build quando i warning superano questa soglia.
    pub max_warnings: Option<usize>,
    /// Se non vuoto, esegue solo le regole con questi id.
    pub only: Vec<String>,
    /// Percorso esplicito del file di config (`--config`).
    pub config_path: Option<PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            quiet: false,
            verbose: false,
            suggestions: true,
            format: None,
            color: Color::Auto,
            max_warnings: None,
            only: Vec::new(),
            config_path: None,
        }
    }
}

/// Come [`run_with`] ma con le opzioni di default.
pub fn run(dir: &str) -> i32 {
    run_with(dir, &Options::default())
}

/// Analizza `dir`, renderizza nel formato scelto, scrive su stdout e ritorna
/// l'**exit code**: `1` se ci sono Error (o se `max_warnings` è superato), `0`
/// altrimenti.
pub fn run_with(dir: &str, opts: &Options) -> i32 {
    let cfg = Config::load(dir, opts.config_path.as_deref());
    let analysis = analyze_with(dir, opts, &cfg);
    let format = opts.format.or(cfg.format).unwrap_or(Format::Pretty);

    let ropts = RenderOpts {
        suggestions: opts.suggestions,
        verbose: opts.verbose,
        quiet: opts.quiet,
        dir: dir.to_string(),
    };
    print(&report::render(format, &analysis, &ropts), opts.color);

    let fail = analysis.errors() > 0
        || opts
            .max_warnings
            .is_some_and(|max| analysis.warnings() > max);
    i32::from(fail)
}

/// Scrive su stdout passando per `anstream`, che abilita i colori ANSI su
/// Windows e li rimuove quando l'output non è un terminale o `--no-color`.
pub fn print(text: &str, color: Color) {
    let mut stream = anstream::AutoStream::new(std::io::stdout().lock(), color.into());
    let _ = write!(stream, "{text}");
}

/// Tabella di tutte le regole (per il comando `rules`).
pub fn render_rules() -> String {
    report::rules_table(&rules::registry())
}

/// Descrizione estesa di una regola (per il comando `explain`); `None` se l'id
/// non esiste.
pub fn render_explain(id: &str) -> Option<String> {
    rules::meta(id).map(|m| report::explain(&m))
}

/// Tutti gli id di regola noti (per messaggi d'errore della CLI).
pub fn rule_ids() -> Vec<&'static str> {
    rules::registry().iter().map(|m| m.id).collect()
}

/// Analizza `dir` con la config trovata e le opzioni di default: la parte
/// testabile del giro, senza stampare nulla.
pub fn analyze(dir: &str) -> Analysis {
    let cfg = Config::load(dir, None);
    analyze_with(dir, &Options::default(), &cfg)
}

/// Cuore dell'analisi: discovery + filtri config/`--only` + lint in parallelo.
fn analyze_with(dir: &str, opts: &Options, cfg: &Config) -> Analysis {
    let start = Instant::now();

    let files: Vec<PathBuf> = discover(dir)
        .into_iter()
        .filter(|p| {
            let rel = p.strip_prefix(dir).unwrap_or(p);
            !cfg.is_ignored(rel)
        })
        .collect();

    let rules: Vec<Box<dyn Rule>> = rules::all()
        .into_iter()
        .filter(|r| !cfg.is_rule_off(r.id()))
        .filter(|r| opts.only.is_empty() || opts.only.iter().any(|o| o == r.id()))
        .collect();

    let mut findings: Vec<Finding> = files
        .par_iter()
        .flat_map_iter(|path| lint_file(path, &rules, cfg))
        .collect();

    // Ordine deterministico: prima per file, poi per regola → output raggruppato.
    findings.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.rule.cmp(b.rule)));

    Analysis {
        pages: files.len(),
        findings,
        elapsed: start.elapsed(),
    }
}

/// Raccoglie ricorsivamente tutti i `.html` sotto `dir`.
///
/// Disabilitiamo i filtri standard di `ignore` (gitignore ecc.) perché vogliamo
/// linterare l'output di build anche quando vive in cartelle gitignorate come
/// `dist/`; teniamo solo `hidden(true)` per saltare `.git` e simili.
fn discover(dir: &str) -> Vec<PathBuf> {
    WalkBuilder::new(dir)
        .standard_filters(false)
        .hidden(true)
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .map(|e| e.into_path())
        .filter(|p| {
            p.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("html"))
        })
        .collect()
}

/// Parsa un file e gli applica le regole attive, agganciando `file`/`source` e
/// applicando l'override di gravità della config.
fn lint_file(path: &Path, rules: &[Box<dyn Rule>], cfg: &Config) -> Vec<Finding> {
    let Ok(source) = std::fs::read_to_string(path) else {
        eprintln!("lightship: impossibile leggere {}", path.display());
        return Vec::new();
    };

    let Ok(dom) = tl::parse(&source, tl::ParserOptions::default()) else {
        eprintln!("lightship: impossibile parsare {}", path.display());
        return Vec::new();
    };

    // Sorgente condiviso fra tutti i finding del file: gli span vi puntano.
    let shared: Arc<str> = Arc::from(source.as_str());
    let mut out = Vec::new();
    for rule in rules {
        for mut finding in rule.check(&dom, &source) {
            finding.severity = cfg.severity_for(finding.rule, finding.severity);
            finding.attach(path.to_path_buf(), shared.clone());
            out.push(finding);
        }
    }
    out
}
