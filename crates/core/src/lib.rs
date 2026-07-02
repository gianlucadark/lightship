mod baseline;
mod config;
mod detect;
mod finding;
mod fix;
mod meta;
mod progress;
mod report;
mod rule;
mod rules;
mod util;

pub use baseline::{BASELINE_FILE, Baseline};
pub use config::{CONFIG_FILE, Config, DEFAULT_CI_WORKFLOW, DEFAULT_CONFIG, ci_workflow};
pub use detect::{Detected, detect_build_dir, detect_framework};
pub use finding::{Finding, Severity};
pub use fix::{Edit, Fix, apply_edits, parse_selection};
pub use meta::{Category, RuleMeta};
pub use report::{Color, Format, RenderOpts};
pub use rule::{Rule, RuleScope};

use ignore::WalkBuilder;
use rayon::prelude::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Risultato dell'analisi di una cartella.
pub struct Analysis {
    pub findings: Vec<Finding>,
    /// File `.html` effettivamente analizzati (esclusi quelli saltati).
    pub pages: usize,
    /// File `.html` saltati perché illeggibili, non parsabili o troppo grandi.
    pub skipped: usize,
    /// Finding già noti soppressi dal baseline.
    pub baselined: usize,
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
    /// Usa glifi solo-ASCII nei report umani.
    pub ascii: bool,
    /// Se impostato, fa fallire la build quando i warning superano questa soglia.
    pub max_warnings: Option<usize>,
    /// Se `true`, qualunque warning fa fallire la build (come `--max-warnings 0`).
    pub error_on_warnings: bool,
    /// Se non vuoto, esegue solo le regole con questi id.
    pub only: Vec<String>,
    /// Se non vuoto, esegue solo le regole di queste categorie (nomi o alias,
    /// es. `a11y`, `seo`). In AND con `only`.
    pub only_categories: Vec<String>,
    /// Preset di regole da eseguire: `recommended` (default), `all`, o il nome di
    /// una categoria (`a11y`/`seo`/…). `None` ⇒ config, poi `recommended`.
    pub preset: Option<String>,
    /// Esegue anche le regole "di documento" sui frammenti/partial (che di
    /// default vengono saltate su file senza `<html>`/`<head>`/doctype).
    pub include_fragments: bool,
    /// Percorso esplicito del file di config (`--config`).
    pub config_path: Option<PathBuf>,
    /// Percorso esplicito di un file di baseline (`--baseline`); se `None` viene
    /// cercato `lightship-baseline.json` nella cartella / cwd.
    pub baseline: Option<PathBuf>,
    /// Se non vuoto, ai fini dell'**exit code** contano solo i finding di queste
    /// categorie (il report li mostra comunque tutti).
    pub fail_on_categories: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            quiet: false,
            verbose: false,
            suggestions: true,
            format: None,
            color: Color::Auto,
            ascii: false,
            max_warnings: None,
            error_on_warnings: false,
            only: Vec::new(),
            only_categories: Vec::new(),
            preset: None,
            include_fragments: false,
            config_path: None,
            baseline: None,
            fail_on_categories: Vec::new(),
        }
    }
}

/// Regole **non** incluse nel preset `recommended` (opt-in): valgono solo con
/// `--preset all` o selezionandole/categoria esplicitamente. Sono regole
/// rumorose o molto opinabili che è meglio non attivare di default.
const NON_RECOMMENDED: &[&str] = &["img-lazy-loading", "og-basic"];

/// Limite di default sulla dimensione di un file `.html`: oltre questa soglia lo
/// saltiamo, per non caricare in memoria artefatti abnormi (bundle, mappe…).
/// Sovrascrivibile con `[analyze] max_file_bytes` nel `lightship.toml`.
pub const DEFAULT_MAX_FILE_BYTES: usize = 8 * 1024 * 1024;

/// Come [`run_with`] ma con le opzioni di default.
pub fn run(dir: &str) -> i32 {
    run_with(dir, &Options::default())
}

/// Analizza `dir`, renderizza nel formato scelto, scrive su stdout e ritorna
/// l'**exit code**: `0` = nessun problema, `1` = Error trovati o soglie warning
/// superate, `2` = errore di configurazione/uso.
pub fn run_with(dir: &str, opts: &Options) -> i32 {
    let cfg = match Config::try_load(dir, opts.config_path.as_deref()) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("lightship: {e}");
            return 2;
        }
    };
    let format = opts.format.or(cfg.format).unwrap_or(Format::Pretty);

    // Lo spinner ha senso solo per i formati umani e quando non siamo in
    // `--quiet`; `Progress` poi lo accende solo se stderr è un terminale.
    let show_progress = !opts.quiet && matches!(format, Format::Pretty | Format::Compact);
    let analysis = analyze_with(dir, opts, &cfg, show_progress, true);

    let ropts = RenderOpts {
        suggestions: opts.suggestions,
        verbose: opts.verbose,
        quiet: opts.quiet,
        dir: if dir == "-" {
            "<stdin>".to_string()
        } else {
            dir.to_string()
        },
        ascii: opts.ascii,
        width: report::term_width(),
    };
    print(&report::render(format, &analysis, &ropts), opts.color);

    // `--fail-on-category`: ai fini dell'exit code contano solo i finding di
    // quelle categorie (il report resta completo).
    let (errors, warnings) = failing_counts(&analysis, &opts.fail_on_categories);

    // Soglie CI: le opzioni della CLI hanno precedenza, poi la config `[ci]`.
    let max_warnings = opts.max_warnings.or(cfg.ci_max_warnings);
    let error_on_warnings = opts.error_on_warnings || cfg.ci_error_on_warnings;
    let fail = errors > 0
        || (error_on_warnings && warnings > 0)
        || max_warnings.is_some_and(|max| warnings > max);
    i32::from(fail)
}

/// Conteggio (errori, warning) rilevante per l'exit code: tutti i finding, o
/// solo quelli delle categorie richieste con `--fail-on-category`.
fn failing_counts(analysis: &Analysis, categories: &[String]) -> (usize, usize) {
    if categories.is_empty() {
        return (analysis.errors(), analysis.warnings());
    }
    let cats: Vec<Category> = categories.iter().filter_map(|s| Category::parse(s)).collect();
    let (mut errors, mut warnings) = (0, 0);
    for f in &analysis.findings {
        if rules::meta(f.rule).is_some_and(|m| cats.contains(&m.category)) {
            match f.severity {
                Severity::Error => errors += 1,
                Severity::Warn => warnings += 1,
            }
        }
    }
    (errors, warnings)
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
    analyze_with(dir, &Options::default(), &cfg, false, true)
}

/// Come [`analyze`] ma con opzioni esplicite (config caricata da `dir`). Utile
/// per pilotare l'analisi da codice/test senza passare dalla CLI.
pub fn analyze_opts(dir: &str, opts: &Options) -> Analysis {
    let cfg = Config::load(dir, opts.config_path.as_deref());
    analyze_with(dir, opts, &cfg, false, true)
}

/// Costruisce un baseline dai finding correnti di `dir`. Non applica un baseline
/// eventualmente già presente: cattura lo stato **completo** attuale.
pub fn build_baseline(dir: &str, opts: &Options) -> Baseline {
    let cfg = Config::load(dir, opts.config_path.as_deref());
    let analysis = analyze_with(dir, opts, &cfg, false, false);
    Baseline::from_findings(&analysis.findings)
}

/// Cuore dell'analisi: discovery + filtri config/`--only` + lint in parallelo.
/// Con `apply_baseline` sottrae i finding già noti (per non far fallire la CI su
/// problemi preesistenti); lo disattiviamo quando *costruiamo* un baseline.
fn analyze_with(
    dir: &str,
    opts: &Options,
    cfg: &Config,
    show_progress: bool,
    apply_baseline: bool,
) -> Analysis {
    let start = Instant::now();

    // Input: `-` = stdin, un file esplicito, oppure una cartella da scoprire.
    let input = Path::new(dir);
    let from_stdin = dir == "-";
    let files: Vec<PathBuf> = if from_stdin {
        Vec::new()
    } else if input.is_file() {
        // File singolo esplicito: niente discovery né filtri ignore (l'utente
        // l'ha chiesto per nome). Abilita editor/pre-commit su un solo file.
        vec![input.to_path_buf()]
    } else {
        // La walk su alberi grossi può richiedere un momento: segno di vita.
        progress::discovering(show_progress, opts.ascii);
        discover(dir)
            .into_iter()
            .filter(|p| {
                let rel = p.strip_prefix(dir).unwrap_or(p);
                !cfg.is_ignored(rel)
            })
            .collect()
    };

    let rules = active_rules(opts, cfg);

    // Le regole "di documento" (title/charset/viewport/h1…) vanno saltate sui
    // frammenti, a meno che l'utente non chieda esplicitamente di includerli.
    let include_fragments = opts.include_fragments || cfg.include_fragments;
    let max_file_bytes = cfg.max_file_bytes.unwrap_or(DEFAULT_MAX_FILE_BYTES);

    let progress = progress::Progress::new(files.len(), show_progress, opts.ascii);
    let results: Vec<LintOutcome> = if from_stdin {
        vec![lint_stdin(&rules, cfg, include_fragments, max_file_bytes)]
    } else {
        files
            .par_iter()
            .map(|path| {
                let r = lint_file(path, &rules, cfg, include_fragments, max_file_bytes);
                progress.tick();
                r
            })
            .collect()
    };
    progress.finish();

    let pages_seen = results.len();
    let skipped = results.iter().filter(|r| r.skipped).count();
    let mut findings: Vec<Finding> = results.into_iter().flat_map(|r| r.findings).collect();

    // Ordine deterministico: prima per file, poi per regola → output raggruppato.
    findings.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.rule.cmp(b.rule)));

    // Sottrazione del baseline: i finding già noti non fanno fallire la build.
    // Contiamo le occorrenze così che una *nuova* copia di un problema noto emerga.
    let mut baselined = 0;
    if apply_baseline && let Some(path) = baseline::resolve(dir, opts.baseline.as_deref()) {
        if let Some(base) = Baseline::load(&path) {
            // I baseline v1 contengono fingerprint derivati dal messaggio: li
            // matchiamo con il fingerprint legacy e suggeriamo di rigenerarli.
            let legacy = base.is_legacy();
            if legacy {
                eprintln!(
                    "lightship: baseline {} uses an old format; run `lightship baseline` to refresh it",
                    path.display()
                );
            }
            let mut remaining = base.fingerprint_counts();
            let before = findings.len();
            findings.retain(|f| {
                let fp = if legacy {
                    f.fingerprint_v1()
                } else {
                    f.fingerprint()
                };
                if let Some(c) = remaining.get_mut(&fp)
                    && *c > 0
                {
                    *c -= 1;
                    return false; // coperto dal baseline
                }
                true
            });
            baselined = before - findings.len();
        } else {
            eprintln!("lightship: could not read baseline {}", path.display());
        }
    }

    Analysis {
        pages: pages_seen - skipped,
        skipped,
        baselined,
        findings,
        elapsed: start.elapsed(),
    }
}

/// Esito dell'analisi di un singolo file: i finding prodotti e se il file è
/// stato saltato (illeggibile / non parsabile / troppo grande).
struct LintOutcome {
    findings: Vec<Finding>,
    skipped: bool,
}

/// `true` se il preset consente la regola `meta`. `recommended` esclude le regole
/// opt-in ([`NON_RECOMMENDED`]); `all` le include tutte; il nome di una categoria
/// filtra a quella categoria; un valore sconosciuto non filtra (è già segnalato
/// altrove) per non nascondere silenziosamente tutte le regole.
fn preset_allows(preset: &str, meta: &RuleMeta) -> bool {
    match preset.trim().to_ascii_lowercase().as_str() {
        "all" => true,
        "recommended" | "" => !NON_RECOMMENDED.contains(&meta.id),
        other => match Category::parse(other) {
            Some(cat) => meta.category == cat,
            None => true,
        },
    }
}

/// I nomi di preset validi (per la validazione lato CLI/config).
pub fn preset_names() -> Vec<&'static str> {
    vec![
        "recommended",
        "all",
        "accessibility",
        "seo",
        "performance",
        "security",
        "correctness",
    ]
}

/// `true` se `s` è un preset valido: `recommended`/`all` o il nome (o alias) di
/// una categoria.
pub fn is_valid_preset(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "recommended" | "all"
    ) || Category::parse(s).is_some()
}

/// L'insieme di regole attive per queste opzioni: applica gli off della config,
/// il filtro `--only`, `--only-category` e il preset. Condiviso da analisi e fix.
fn active_rules(opts: &Options, cfg: &Config) -> Vec<Box<dyn Rule>> {
    let only_categories: Vec<Category> = opts
        .only_categories
        .iter()
        .filter_map(|c| Category::parse(c))
        .collect();
    let preset = opts
        .preset
        .clone()
        .or_else(|| cfg.preset.clone())
        .unwrap_or_else(|| "recommended".to_string());
    // Una selezione esplicita (per id o categoria) ha la precedenza sul preset.
    let explicit_selection = !opts.only.is_empty() || !only_categories.is_empty();

    rules::all()
        .into_iter()
        .filter(|r| !cfg.is_rule_off(r.id()))
        .filter(|r| opts.only.is_empty() || opts.only.iter().any(|o| o == r.id()))
        .filter(|r| only_categories.is_empty() || only_categories.contains(&r.meta().category))
        .filter(|r| explicit_selection || preset_allows(&preset, &r.meta()))
        .collect()
}

/// Raccoglie tutti i fix sicuri proponibili su `dir` (stessa discovery/filtri
/// dell'analisi), con file e riga/colonna già agganciati e ordinati per file.
pub fn collect_fixes(dir: &str, opts: &Options) -> Vec<Fix> {
    let cfg = Config::load(dir, opts.config_path.as_deref());
    let rules = active_rules(opts, &cfg);
    let include_fragments = opts.include_fragments || cfg.include_fragments;
    let max_file_bytes = cfg.max_file_bytes.unwrap_or(DEFAULT_MAX_FILE_BYTES);

    let files: Vec<PathBuf> = discover(dir)
        .into_iter()
        .filter(|p| !cfg.is_ignored(p.strip_prefix(dir).unwrap_or(p)))
        .collect();

    let mut fixes: Vec<Fix> = files
        .par_iter()
        .flat_map_iter(|path| file_fixes(path, &rules, include_fragments, max_file_bytes))
        .collect();
    fixes.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.column.cmp(&b.column))
    });
    fixes
}

/// Fix proponibili su un singolo file (rispettando il gating dei frammenti).
fn file_fixes(
    path: &Path,
    rules: &[Box<dyn Rule>],
    include_fragments: bool,
    max_file_bytes: usize,
) -> Vec<Fix> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    if bytes.len() > max_file_bytes {
        return Vec::new();
    }
    let source = String::from_utf8_lossy(&bytes).into_owned();
    let Ok(dom) = tl::parse(&source, tl::ParserOptions::default()) else {
        return Vec::new();
    };
    let run_document = include_fragments || util::is_full_document(&dom, &source);

    let shared: Arc<str> = Arc::from(source.as_str());
    let mut out = Vec::new();
    for rule in rules {
        if rule.scope() == RuleScope::Document && !run_document {
            continue;
        }
        for mut fix in rule.fixes(&dom, &source) {
            fix.attach(path.to_path_buf(), shared.clone());
            out.push(fix);
        }
    }
    out
}

/// Applica gli `edit` selezionati ai rispettivi file (raggruppandoli per file) e
/// scrive su disco. Ritorna quanti file sono stati modificati o il primo errore.
pub fn apply_fixes(fixes: &[&Fix]) -> std::io::Result<usize> {
    use std::collections::BTreeMap;
    let mut by_file: BTreeMap<&Path, Vec<fix::Edit>> = BTreeMap::new();
    for f in fixes {
        by_file
            .entry(f.file.as_path())
            .or_default()
            .push(f.edit.clone());
    }
    let count = by_file.len();
    for (path, edits) in by_file {
        let source = std::fs::read_to_string(path)?;
        let patched = fix::apply_edits(&source, &edits);
        std::fs::write(path, patched)?;
    }
    Ok(count)
}

/// Cartelle pesanti che non sono mai output di build: le potiamo *prima* di
/// entrarci, così non sprechiamo tempo a camminare migliaia di file di
/// dipendenze quando l'utente punta lightship alla root del progetto.
const PRUNED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    ".cache",
    "vendor",
    "bower_components",
    ".venv",
    "__pycache__",
];

/// Raccoglie ricorsivamente tutti i `.html` sotto `dir`.
///
/// Disabilitiamo i filtri standard di `ignore` (gitignore ecc.) perché vogliamo
/// linterare l'output di build anche quando vive in cartelle gitignorate come
/// `dist/`; teniamo solo `hidden(true)` per saltare `.git` e simili. In più
/// potiamo esplicitamente le cartelle pesanti (`node_modules`, `vendor`, …) via
/// [`PRUNED_DIRS`], così non ci entriamo nemmeno.
fn discover(dir: &str) -> Vec<PathBuf> {
    WalkBuilder::new(dir)
        .standard_filters(false)
        .hidden(true)
        .filter_entry(|e| {
            // Pota le directory note: i file restano accettati.
            if e.file_type().is_some_and(|t| t.is_dir()) {
                let name = e.file_name().to_string_lossy();
                !PRUNED_DIRS.iter().any(|d| name.eq_ignore_ascii_case(d))
            } else {
                true
            }
        })
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
/// applicando l'override di gravità della config. Ritorna anche se il file è
/// stato saltato, così l'orchestratore può contarlo separatamente dalle pagine.
fn lint_file(
    path: &Path,
    rules: &[Box<dyn Rule>],
    cfg: &Config,
    include_fragments: bool,
    max_file_bytes: usize,
) -> LintOutcome {
    let skip = |msg: &str| {
        eprintln!("lightship: {msg} {}", path.display());
        LintOutcome {
            findings: Vec::new(),
            skipped: true,
        }
    };

    let Ok(bytes) = std::fs::read(path) else {
        return skip("could not read");
    };
    if bytes.len() > max_file_bytes {
        return skip("skipping file larger than the size limit:");
    }
    // Lettura tollerante: i file di build possono avere byte non-UTF-8 (es.
    // contenuti latin-1); li sostituiamo invece di saltare l'intero file.
    let source = String::from_utf8_lossy(&bytes).into_owned();
    lint_source(path, &source, rules, cfg, include_fragments)
}

/// Legge una pagina da **stdin** e la linta come fosse un file `<stdin>`.
/// Frammento o documento completo vengono distinti come per i file su disco.
fn lint_stdin(
    rules: &[Box<dyn Rule>],
    cfg: &Config,
    include_fragments: bool,
    max_file_bytes: usize,
) -> LintOutcome {
    use std::io::Read;
    let skipped = LintOutcome {
        findings: Vec::new(),
        skipped: true,
    };
    let mut bytes = Vec::new();
    if std::io::stdin().read_to_end(&mut bytes).is_err() {
        eprintln!("lightship: could not read stdin");
        return skipped;
    }
    if bytes.len() > max_file_bytes {
        eprintln!("lightship: skipping stdin larger than the size limit");
        return skipped;
    }
    let source = String::from_utf8_lossy(&bytes).into_owned();
    lint_source(Path::new("<stdin>"), &source, rules, cfg, include_fragments)
}

/// Applica le regole attive a un sorgente già letto, agganciando `path`/`source`
/// e l'override di gravità della config. Cuore condiviso fra file e stdin.
fn lint_source(
    path: &Path,
    source: &str,
    rules: &[Box<dyn Rule>],
    cfg: &Config,
    include_fragments: bool,
) -> LintOutcome {
    let Ok(dom) = tl::parse(source, tl::ParserOptions::default()) else {
        eprintln!("lightship: could not parse {}", path.display());
        return LintOutcome {
            findings: Vec::new(),
            skipped: true,
        };
    };

    // Su un frammento (niente `<html>`/`<head>`/doctype) le regole "di documento"
    // darebbero falsi positivi, quindi le saltiamo salvo richiesta esplicita.
    let run_document_rules = include_fragments || util::is_full_document(&dom, source);

    // Sorgente condiviso fra tutti i finding del file: gli span vi puntano.
    let shared: Arc<str> = Arc::from(source);
    // Indice delle righe costruito una sola volta per file e riusato da tutti i
    // finding per calcolare riga/colonna senza riscandire il sorgente.
    let index = finding::LineIndex::new(source);
    let mut out = Vec::new();
    for rule in rules {
        if rule.scope() == RuleScope::Document && !run_document_rules {
            continue;
        }
        for mut finding in rule.check(&dom, source) {
            finding.severity = cfg.severity_for(finding.rule, finding.severity);
            finding.attach_with(path.to_path_buf(), shared.clone(), &index);
            out.push(finding);
        }
    }
    LintOutcome {
        findings: out,
        skipped: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `discover` non deve scendere in `node_modules` & co.
    #[test]
    fn discover_skips_heavy_dirs() {
        let base = std::env::temp_dir().join(format!("lightship-discover-{}", std::process::id()));
        let nm = base.join("node_modules").join("pkg");
        std::fs::create_dir_all(&nm).unwrap();
        std::fs::write(base.join("index.html"), "<html></html>").unwrap();
        std::fs::write(nm.join("dep.html"), "<html></html>").unwrap();

        let found = discover(&base.to_string_lossy());

        std::fs::remove_dir_all(&base).ok();

        assert_eq!(found.len(), 1, "atteso solo index.html, trovati: {found:?}");
        assert!(found[0].ends_with("index.html"));
    }

    /// Un path di file esplicito viene lintato direttamente, senza discovery.
    #[test]
    fn analyze_accetta_un_file_singolo() {
        let base = std::env::temp_dir().join(format!("lightship-singlefile-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let file = base.join("page.html");
        std::fs::write(&file, r#"<div><img src="a.png" width="1" height="1"></div>"#).unwrap();
        // Un secondo file nella stessa cartella NON deve essere analizzato.
        std::fs::write(base.join("other.html"), r#"<div><img src="b.png"></div>"#).unwrap();

        let analysis = analyze(&file.to_string_lossy());
        std::fs::remove_dir_all(&base).ok();

        assert_eq!(analysis.pages, 1);
        assert_eq!(analysis.findings.len(), 1);
        assert_eq!(analysis.findings[0].rule, "img-alt");
    }

    /// `--fail-on-category` restringe i conteggi che decidono l'exit code.
    #[test]
    fn failing_counts_filtra_per_categoria() {
        let mut a11y = Finding::new("img-alt", Severity::Error, "m", None);
        a11y.file = PathBuf::from("a.html");
        let mut seo = Finding::new("meta-charset", Severity::Warn, "m", None);
        seo.file = PathBuf::from("a.html");
        let analysis = Analysis {
            pages: 1,
            skipped: 0,
            baselined: 0,
            findings: vec![a11y, seo],
            elapsed: Duration::from_millis(0),
        };

        assert_eq!(failing_counts(&analysis, &[]), (1, 1));
        assert_eq!(failing_counts(&analysis, &["seo".to_string()]), (0, 1));
        assert_eq!(failing_counts(&analysis, &["a11y".to_string()]), (1, 0));
        // Categoria senza finding: non fallisce nulla.
        assert_eq!(failing_counts(&analysis, &["security".to_string()]), (0, 0));
    }
}
