mod finding;
mod report;
mod rule;
mod rules;
mod util;

pub use finding::{Finding, Severity};
pub use rule::Rule;

use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// Risultato dell'analisi di una cartella.
pub struct Analysis {
    pub findings: Vec<Finding>,
    pub pages: usize,
}

/// Opzioni di esecuzione della CLI/libreria.
#[derive(Debug, Default, Clone, Copy)]
pub struct Options {
    /// Stampa solo la riga di riepilogo, non i singoli finding.
    pub quiet: bool,
}

/// Come [`run_with`] ma con le opzioni di default.
pub fn run(dir: &str) -> usize {
    run_with(dir, Options::default())
}

/// Cammina `dir`, lint di ogni `.html` in parallelo, stampa i finding raggruppati
/// per file e un riepilogo. Ritorna il numero di finding di gravità `Error`.
pub fn run_with(dir: &str, opts: Options) -> usize {
    let start = Instant::now();
    let analysis = analyze(dir);

    if !opts.quiet {
        print!("{}", report::render(&analysis.findings));
    }

    let errors = analysis
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warns = analysis.findings.len() - errors;

    println!(
        "{} pagine · {}ms · {} error, {} warn",
        analysis.pages,
        start.elapsed().as_millis(),
        errors,
        warns
    );

    errors
}

/// Analizza `dir` senza stampare nulla: la parte testabile del giro.
pub fn analyze(dir: &str) -> Analysis {
    let files = discover(dir);
    let rules = rules::all();

    let mut findings: Vec<Finding> = files
        .par_iter()
        .flat_map_iter(|path| lint_file(path, &rules))
        .collect();

    // Ordine deterministico: prima per file, poi per regola → output raggruppato.
    findings.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.rule.cmp(b.rule)));

    Analysis {
        pages: files.len(),
        findings,
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

/// Parsa un file e gli applica tutte le regole, impostando `file` e `source`
/// sui finding (così il report può mostrare lo snippet di codice reale).
fn lint_file(path: &Path, rules: &[Box<dyn Rule>]) -> Vec<Finding> {
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
            finding.file = path.to_path_buf();
            finding.source = shared.clone();
            out.push(finding);
        }
    }
    out
}
