use crate::Analysis;
use crate::finding::Severity;
use std::fmt::Write;

/// Workflow command di GitHub Actions: ogni finding diventa un'annotazione
/// inline sul file/riga. Vedi:
/// https://docs.github.com/actions/using-workflows/workflow-commands-for-github-actions
pub fn render(analysis: &Analysis) -> String {
    let mut out = String::new();
    for f in &analysis.findings {
        let kind = match f.severity {
            Severity::Error => "error",
            Severity::Warn => "warning",
        };
        let file = f.file.display().to_string().replace('\\', "/");
        let mut props = format!("file={}", escape_prop(&file));
        if let Some(l) = f.line {
            let _ = write!(props, ",line={l}");
        }
        if let Some(c) = f.column {
            let _ = write!(props, ",col={c}");
        }
        let _ = write!(props, ",title=lightship/{}", f.rule);
        let _ = writeln!(out, "::{kind} {props}::{}", escape_data(&f.message));
    }
    out
}

/// Escape per i *valori dei dati* dei workflow command.
fn escape_data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escape per le *proprietà* (in più virgola e due punti).
fn escape_prop(s: &str) -> String {
    escape_data(s).replace(',', "%2C").replace(':', "%3A")
}
