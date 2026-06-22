use lightship_core::Options;
use std::process::ExitCode;

const HELP: &str = "\
lightship — linter statico per l'output HTML di build

USO:
    lightship [OPZIONI] [CARTELLA]

ARGOMENTI:
    CARTELLA    Cartella da analizzare ricorsivamente (default: cartella corrente)

OPZIONI:
    -q, --quiet      Stampa solo la riga di riepilogo, non i singoli finding
    -h, --help       Mostra questo aiuto
    -V, --version    Mostra la versione

USCITA:
    0 se non ci sono error, 1 se almeno un finding ha gravità Error.";

fn main() -> ExitCode {
    let mut dir: Option<String> = None;
    let mut opts = Options::default();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("lightship {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "-q" | "--quiet" => opts.quiet = true,
            other if other.starts_with('-') => {
                eprintln!("lightship: opzione sconosciuta '{other}'\n");
                eprintln!("{HELP}");
                return ExitCode::FAILURE;
            }
            path => dir = Some(path.to_string()),
        }
    }

    let dir = dir.unwrap_or_else(|| ".".to_string());

    if lightship_core::run_with(&dir, opts) > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
