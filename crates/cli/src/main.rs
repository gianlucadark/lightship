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
    about = "🛳  Linter statico per l'output HTML di build (a11y / SEO / performance)",
    long_about = "Lightship analizza i file .html già buildati (qualunque framework), \
                  senza browser, e fa fallire la build su problemi di accessibilità, \
                  SEO o performance.",
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
    /// Analizza una cartella e segnala i problemi (comando di default).
    #[command(visible_aliases = ["check", "scan"])]
    Analyze(AnalyzeArgs),

    /// Elenca tutte le regole con gravità e descrizione.
    Rules,

    /// Mostra il dettaglio di una regola: cosa controlla, come correggere, esempi.
    Explain {
        /// Id della regola, es. `img-alt`.
        rule: String,
    },

    /// Crea un file `lightship.toml` di default nella cartella indicata.
    Init {
        /// Cartella in cui creare il file (default: corrente).
        #[arg(default_value = ".")]
        dir: String,
    },
}

#[derive(Args)]
struct AnalyzeArgs {
    /// Cartella da analizzare ricorsivamente.
    #[arg(default_value = ".")]
    dir: String,

    /// Stampa solo il pannello di riepilogo, non i singoli finding.
    #[arg(short, long)]
    quiet: bool,

    /// Output più dettagliato (non tronca gli snippet lunghi).
    #[arg(short, long)]
    verbose: bool,

    /// Formato di output.
    #[arg(long, value_enum)]
    format: Option<FormatArg>,

    /// Disattiva i colori ANSI.
    #[arg(long = "no-color")]
    no_color: bool,

    /// Non mostrare la riga 💡 con il suggerimento di fix.
    #[arg(long = "no-suggestions")]
    no_suggestions: bool,

    /// Fa fallire la build se i warning superano questa soglia.
    #[arg(long = "max-warnings", value_name = "N")]
    max_warnings: Option<usize>,

    /// Esegui solo queste regole (separate da virgola).
    #[arg(long, value_delimiter = ',', value_name = "REGOLE")]
    only: Vec<String>,

    /// Percorso esplicito del file di config.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Ri-analizza automaticamente quando i file cambiano (Ctrl-C per uscire).
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
            color: if self.no_color {
                Color::Never
            } else {
                Color::Auto
            },
            max_warnings: self.max_warnings,
            only: self.only.clone(),
            config_path: self.config.clone(),
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
    }
}

fn run_analyze(a: &AnalyzeArgs) -> ExitCode {
    let opts = a.to_options();
    if a.watch {
        watch(&a.dir, &opts);
        ExitCode::SUCCESS
    } else {
        let code = lightship_core::run_with(&a.dir, &opts);
        ExitCode::from(code as u8)
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
                "lightship: regola sconosciuta '{rule}'.\nRegole disponibili: {}",
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
            "lightship: {} esiste già, non lo sovrascrivo",
            path.display()
        );
        return ExitCode::FAILURE;
    }
    match std::fs::write(&path, lightship_core::DEFAULT_CONFIG) {
        Ok(()) => {
            println!("Creato {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("lightship: impossibile creare {}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
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
            eprintln!("lightship: impossibile avviare il watch: {e}");
            return;
        }
    };
    if let Err(e) = watcher.watch(Path::new(dir), RecursiveMode::Recursive) {
        eprintln!("lightship: impossibile osservare {dir}: {e}");
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
    println!("\n{}", "  (watch attivo · Ctrl-C per uscire)");
}
