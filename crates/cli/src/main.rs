use clap::{Args, Parser, Subcommand, ValueEnum};
use lightship_core::{Color, Format, Options};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;

/// Linter statico per l'output HTML di build (a11y / SEO / performance).
#[derive(Parser)]
#[command(
    name = "lightship",
    version,
    about = "🛳  Static linter for built HTML output (a11y / SEO / performance)",
    long_about = "Lightship analyzes your already-built .html files (any framework), \
                  without a browser, and fails the build on accessibility, SEO or \
                  performance problems.\n\n\
                  Run with no arguments to auto-detect your build output folder \
                  (dist, build, out, _site, public...).",
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Argomenti di `analyze` usati quando non si passa un sottocomando.
    #[command(flatten)]
    analyze: AnalyzeArgs,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze a folder and report problems (the default command).
    #[command(visible_aliases = ["check", "scan"])]
    Analyze(AnalyzeArgs),

    /// List all rules with their severity and description.
    Rules,

    /// Show a rule in detail: what it checks, how to fix it, examples.
    Explain {
        /// Rule id, e.g. `img-alt`.
        rule: String,
    },

    /// Create a default `lightship.toml` (detects your framework).
    Init {
        /// Folder to create the file in (default: current).
        #[arg(default_value = ".")]
        dir: String,
    },

    /// Scaffold a GitHub Actions workflow that runs Lightship in CI.
    Ci {
        /// Build output folder to lint in CI (default: auto-detected).
        dir: Option<String>,
    },

    /// Freeze current findings into a baseline so CI fails only on new issues.
    Baseline(AnalyzeArgs),

    /// Interactively apply safe automatic fixes (choose which, or all).
    Fix(FixArgs),
}

#[derive(Args)]
struct FixArgs {
    /// Folder to fix (default: auto-detected build output).
    dir: Option<String>,

    /// Apply every proposed fix without prompting.
    #[arg(short, long)]
    all: bool,

    /// Show what would change without writing any file.
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Only propose fixes for these rules (comma-separated).
    #[arg(long, value_delimiter = ',', value_name = "RULES")]
    only: Vec<String>,

    /// Also fix document-level rules on HTML fragments/partials.
    #[arg(long = "include-fragments")]
    include_fragments: bool,

    /// Disable ANSI colors.
    #[arg(long = "no-color")]
    no_color: bool,

    /// Explicit path to the config file.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
}

impl FixArgs {
    fn to_options(&self) -> Options {
        Options {
            // Il fix è un'azione esplicita: considera tutte le regole fixabili
            // (incluse le opt-in), poi l'utente sceglie quali applicare.
            preset: Some("all".to_string()),
            only: self.only.clone(),
            include_fragments: self.include_fragments,
            config_path: self.config.clone(),
            ..Options::default()
        }
    }
}

#[derive(Args)]
struct AnalyzeArgs {
    /// Folder to analyze recursively (default: auto-detected build output).
    dir: Option<String>,

    /// Print only the summary panel, not individual findings.
    #[arg(short, long)]
    quiet: bool,

    /// More detailed output (does not truncate long snippets).
    #[arg(short, long)]
    verbose: bool,

    /// Output format.
    #[arg(long, value_enum)]
    format: Option<FormatArg>,

    /// When to use ANSI colors.
    #[arg(long, value_enum, value_name = "WHEN")]
    color: Option<ColorArg>,

    /// Disable ANSI colors (alias for --color never).
    #[arg(long = "no-color")]
    no_color: bool,

    /// Use ASCII-only glyphs (no box-drawing / unicode markers).
    #[arg(long)]
    ascii: bool,

    /// Hide the 💡 fix suggestion line.
    #[arg(long = "no-suggestions")]
    no_suggestions: bool,

    /// Fail the build if warnings exceed this threshold.
    #[arg(long = "max-warnings", value_name = "N")]
    max_warnings: Option<usize>,

    /// Fail the build on any warning (like --max-warnings 0).
    #[arg(long = "error-on-warnings")]
    error_on_warnings: bool,

    /// Run only these rules (comma-separated).
    #[arg(long, value_delimiter = ',', value_name = "RULES")]
    only: Vec<String>,

    /// Run only rules in these categories (comma-separated): accessibility (a11y),
    /// seo, performance, security, correctness.
    #[arg(
        long = "only-category",
        value_delimiter = ',',
        value_name = "CATEGORIES"
    )]
    only_category: Vec<String>,

    /// Rule set to run: recommended (default), all, or a category name.
    #[arg(long, value_name = "PRESET")]
    preset: Option<String>,

    /// Also run document-level rules (title, charset, viewport, single-h1...)
    /// on HTML fragments/partials that have no <html>/<head>.
    #[arg(long = "include-fragments")]
    include_fragments: bool,

    /// Explicit path to the config file.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Suppress findings listed in this baseline file (default: auto-detected
    /// lightship-baseline.json). Only new issues fail the build.
    #[arg(long, value_name = "PATH")]
    baseline: Option<PathBuf>,

    /// Re-analyze automatically when files change (Ctrl-C to quit).
    #[arg(long)]
    watch: bool,
}

#[derive(Copy, Clone, ValueEnum)]
enum FormatArg {
    Pretty,
    Compact,
    Json,
    Sarif,
    Github,
}

#[derive(Copy, Clone, ValueEnum)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

impl From<ColorArg> for Color {
    fn from(c: ColorArg) -> Self {
        match c {
            ColorArg::Auto => Color::Auto,
            ColorArg::Always => Color::Always,
            ColorArg::Never => Color::Never,
        }
    }
}

impl From<FormatArg> for Format {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Pretty => Format::Pretty,
            FormatArg::Compact => Format::Compact,
            FormatArg::Json => Format::Json,
            FormatArg::Sarif => Format::Sarif,
            FormatArg::Github => Format::Github,
        }
    }
}

impl AnalyzeArgs {
    fn to_options(&self) -> Options {
        Options {
            quiet: self.quiet,
            verbose: self.verbose,
            suggestions: !self.no_suggestions,
            format: self.format.map(Format::from),
            // Priorità: --no-color (esplicito spegnimento) > --color > default Auto.
            color: if self.no_color {
                Color::Never
            } else {
                self.color.map(Color::from).unwrap_or(Color::Auto)
            },
            ascii: self.ascii,
            max_warnings: self.max_warnings,
            error_on_warnings: self.error_on_warnings,
            only: self.only.clone(),
            only_categories: self.only_category.clone(),
            preset: self.preset.clone(),
            include_fragments: self.include_fragments,
            config_path: self.config.clone(),
            baseline: self.baseline.clone(),
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        None => run_analyze(&cli.analyze),
        Some(Command::Analyze(a)) => run_analyze(&a),
        Some(Command::Rules) => {
            lightship_core::print(&lightship_core::render_rules(), Color::Auto);
            ExitCode::SUCCESS
        }
        Some(Command::Explain { rule }) => explain(&rule),
        Some(Command::Init { dir }) => init(&dir),
        Some(Command::Ci { dir }) => ci(dir.as_deref()),
        Some(Command::Baseline(a)) => baseline(&a),
        Some(Command::Fix(a)) => fix(&a),
    }
}

fn run_analyze(a: &AnalyzeArgs) -> ExitCode {
    if let Some(p) = &a.preset
        && !lightship_core::is_valid_preset(p)
    {
        eprintln!(
            "lightship: unknown preset '{p}'.\nAvailable presets: {}",
            lightship_core::preset_names().join(", ")
        );
        return ExitCode::FAILURE;
    }

    let opts = a.to_options();

    // Senza cartella esplicita proviamo a rilevare la cartella di build, così
    // l'utente medio può lanciare `lightship` e basta.
    let dir = match &a.dir {
        Some(d) => d.clone(),
        None => match resolve_auto_dir() {
            Some(d) => d,
            None => return ExitCode::SUCCESS,
        },
    };

    if a.watch {
        watch(&dir, &opts);
        ExitCode::SUCCESS
    } else {
        let code = lightship_core::run_with(&dir, &opts);
        ExitCode::from(code as u8)
    }
}

/// Rileva la cartella di build da analizzare quando non è stata passata.
/// Le note vanno su **stderr** per non sporcare stdout nei formati macchina
/// (`--format json|sarif|github`). `None` ⇒ niente trovato, già segnalato.
fn resolve_auto_dir() -> Option<String> {
    match lightship_core::detect_build_dir(Path::new(".")) {
        Some(d) => {
            let shown = d.dir.display();
            match d.framework {
                Some(fw) => eprintln!("lightship: analyzing detected output \"{shown}\" ({fw})"),
                None => eprintln!("lightship: analyzing detected output \"{shown}\""),
            }
            Some(d.dir.to_string_lossy().into_owned())
        }
        None => {
            eprintln!(
                "lightship: no build output folder found.\n\
                 Build your project first, then point lightship at the output folder, e.g.:\n  \
                 lightship dist\n\
                 Common output folders: dist, build, out, _site, public."
            );
            None
        }
    }
}

fn explain(rule: &str) -> ExitCode {
    match lightship_core::render_explain(rule) {
        Some(text) => {
            lightship_core::print(&text, Color::Auto);
            ExitCode::SUCCESS
        }
        None => {
            eprintln!(
                "lightship: unknown rule '{rule}'.\nAvailable rules: {}",
                lightship_core::rule_ids().join(", ")
            );
            ExitCode::FAILURE
        }
    }
}

fn init(dir: &str) -> ExitCode {
    let path = Path::new(dir).join(lightship_core::CONFIG_FILE);
    if path.exists() {
        eprintln!(
            "lightship: {} already exists, not overwriting",
            path.display()
        );
        return ExitCode::FAILURE;
    }

    // Rileva il framework per un'intestazione utile e i "next steps".
    let detected = lightship_core::detect_framework(Path::new(dir));
    let header = match detected {
        Some((fw, out)) => {
            format!("# Detected {fw} project — build output is usually in \"{out}/\".\n")
        }
        None => String::new(),
    };
    let body = format!("{header}{}", lightship_core::DEFAULT_CONFIG);

    match std::fs::write(&path, body) {
        Ok(()) => {
            println!("Created {}", path.display());
            let out_dir = detected.map(|(_, o)| o).unwrap_or("dist");
            println!("\nNext steps:");
            println!("  1. Build your project (e.g. npm run build)");
            println!("  2. Lint the output:  lightship {out_dir}");
            println!("  3. Add CI:           lightship ci");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("lightship: could not create {}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
}

/// Scaffolda `.github/workflows/lightship.yml`. La cartella di build è quella
/// esplicita, altrimenti quella tipica del framework rilevato, altrimenti
/// `dist`. Non sovrascrive un workflow già esistente.
fn ci(dir_arg: Option<&str>) -> ExitCode {
    let build_dir = dir_arg
        .map(str::to_string)
        .or_else(|| lightship_core::detect_framework(Path::new(".")).map(|(_, o)| o.to_string()))
        .unwrap_or_else(|| "dist".to_string());

    let workflow_dir = Path::new(".github").join("workflows");
    let path = workflow_dir.join("lightship.yml");
    if path.exists() {
        eprintln!(
            "lightship: {} already exists, not overwriting",
            path.display()
        );
        return ExitCode::FAILURE;
    }
    if let Err(e) = std::fs::create_dir_all(&workflow_dir) {
        eprintln!(
            "lightship: could not create {}: {e}",
            workflow_dir.display()
        );
        return ExitCode::FAILURE;
    }
    match std::fs::write(&path, lightship_core::ci_workflow(&build_dir)) {
        Ok(()) => {
            println!("Created {}", path.display());
            println!(
                "It builds your project and runs `lightship {build_dir}` on every push and PR."
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("lightship: could not create {}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
}

/// Costruisce e scrive il file di baseline con i finding correnti.
fn baseline(a: &AnalyzeArgs) -> ExitCode {
    let opts = a.to_options();
    let dir = match &a.dir {
        Some(d) => d.clone(),
        None => match resolve_auto_dir() {
            Some(d) => d,
            None => return ExitCode::FAILURE,
        },
    };

    let base = lightship_core::build_baseline(&dir, &opts);
    let count = base.entries.len();
    let path = a
        .baseline
        .clone()
        .unwrap_or_else(|| Path::new(&dir).join(lightship_core::BASELINE_FILE));

    match std::fs::write(&path, base.to_json()) {
        Ok(()) => {
            println!(
                "Wrote {} with {count} baselined {}.",
                path.display(),
                if count == 1 { "issue" } else { "issues" }
            );
            println!("Future runs will only fail on new issues.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("lightship: could not write {}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
}

/// Applica i fix automatici sicuri, in modo interattivo (o tutti con `--all`).
fn fix(a: &FixArgs) -> ExitCode {
    use std::io::Write;

    let color = if a.no_color {
        Color::Never
    } else {
        Color::Auto
    };
    let opts = a.to_options();
    let dir = match &a.dir {
        Some(d) => d.clone(),
        None => match resolve_auto_dir() {
            Some(d) => d,
            None => return ExitCode::FAILURE,
        },
    };

    let fixes = lightship_core::collect_fixes(&dir, &opts);
    if fixes.is_empty() {
        println!("Nothing to auto-fix. ✓");
        return ExitCode::SUCCESS;
    }

    // Lista numerata, con posizione, regola, cosa fa e anteprima della modifica.
    lightship_core::print(&render_fix_list(&fixes), color);

    // Selezione: tutti con --all o --dry-run (anteprima completa), altrimenti
    // chiediamo su stdin quali applicare.
    let selected: Vec<usize> = if a.all || a.dry_run {
        (0..fixes.len()).collect()
    } else {
        print!("\nSelect fixes to apply [e.g. 1,3 or 2-4 · 'a' all · 'q' quit]: ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            eprintln!("lightship: could not read input");
            return ExitCode::FAILURE;
        }
        match lightship_core::parse_selection(&line, fixes.len()) {
            Some(sel) => sel,
            None => {
                println!("No changes made.");
                return ExitCode::SUCCESS;
            }
        }
    };

    let chosen: Vec<&lightship_core::Fix> = selected.iter().map(|&i| &fixes[i]).collect();

    if a.dry_run {
        let mut files: Vec<String> = chosen
            .iter()
            .map(|f| f.file.display().to_string())
            .collect();
        files.sort();
        files.dedup();
        println!(
            "\nDry run: {} fix(es) would be applied across {} file(s):",
            chosen.len(),
            files.len()
        );
        for f in files {
            println!("  {f}");
        }
        return ExitCode::SUCCESS;
    }

    match lightship_core::apply_fixes(&chosen) {
        Ok(files) => {
            println!("\nApplied {} fix(es) across {files} file(s).", chosen.len());
            println!("Re-run `lightship {dir}` to verify.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("lightship: could not apply fixes: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Renderizza la lista numerata dei fix proposti.
fn render_fix_list(fixes: &[lightship_core::Fix]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "\n{} auto-fixable issue(s):\n", fixes.len());
    for (i, f) in fixes.iter().enumerate() {
        let loc = match (f.line, f.column) {
            (Some(l), Some(c)) => format!("{}:{l}:{c}", f.file.display()),
            _ => f.file.display().to_string(),
        };
        let _ = writeln!(out, "  [{}] {}  ({})", i + 1, f.message, f.rule);
        let _ = writeln!(out, "       {loc}");
        let _ = writeln!(out, "       → insert: {}", f.preview);
    }
    out
}

/// Ri-analizza a ogni cambiamento dei file sotto `dir`, fino a Ctrl-C.
fn watch(dir: &str, opts: &Options) {
    use notify::{RecursiveMode, Watcher};

    clear_and_run(dir, opts);

    let (tx, rx) = mpsc::channel();
    let mut watcher = match notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    }) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("lightship: could not start watch mode: {e}");
            return;
        }
    };
    if let Err(e) = watcher.watch(Path::new(dir), RecursiveMode::Recursive) {
        eprintln!("lightship: could not watch {dir}: {e}");
        return;
    }

    while rx.recv().is_ok() {
        // Debounce: svuota gli eventi accumulati nei prossimi 200ms.
        while rx.recv_timeout(Duration::from_millis(200)).is_ok() {}
        clear_and_run(dir, opts);
    }
}

fn clear_and_run(dir: &str, opts: &Options) {
    // Pulisce lo schermo e riposiziona il cursore in alto a sinistra.
    print!("\x1B[2J\x1B[H");
    let _ = lightship_core::run_with(dir, opts);
    println!("\n  (watching · Ctrl-C to quit)");
}
