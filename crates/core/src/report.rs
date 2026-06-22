use crate::finding::{Finding, Severity};
use miette::{
    Diagnostic, GraphicalReportHandler, LabeledSpan, NamedSource, Severity as MietteSeverity,
    SourceCode,
};
use std::fmt;

/// Adatta un `Finding` a `miette::Diagnostic` per il rendering "fancy".
///
/// Usiamo come "source code" il sorgente **completo** del file e mettiamo la
/// label sullo span reale del tag: miette estrae lo snippet di codice vero e ci
/// stampa sopra il numero di riga e colonna corretti.
#[derive(Debug)]
struct FindingDiagnostic {
    finding: Finding,
    source: NamedSource<String>,
}

impl fmt::Display for FindingDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.finding.message)
    }
}

impl std::error::Error for FindingDiagnostic {}

impl Diagnostic for FindingDiagnostic {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new(self.finding.rule))
    }

    fn severity(&self) -> Option<MietteSeverity> {
        Some(match self.finding.severity {
            Severity::Error => MietteSeverity::Error,
            Severity::Warn => MietteSeverity::Warning,
        })
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        // Solo i finding con uno span hanno un pezzo di codice da mostrare; per
        // quelli "di documento" stampiamo il solo messaggio.
        self.finding.span.map(|_| &self.source as &dyn SourceCode)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let (offset, len) = self.finding.span?;
        if len == 0 {
            return None;
        }
        let span = LabeledSpan::new(Some("qui".to_string()), offset, len);
        Some(Box::new(std::iter::once(span)))
    }
}

/// Renderizza i finding (già ordinati per file) in una stringa leggibile,
/// con un'intestazione per ogni file che cambia.
pub fn render(findings: &[Finding]) -> String {
    // `with_context_lines(0)` → mostriamo solo la riga del tag incriminato, senza
    // righe di contorno: output pulito e focalizzato, stile compilatore Rust.
    let handler = GraphicalReportHandler::new().with_context_lines(0);
    let mut out = String::new();
    let mut current: Option<&std::path::Path> = None;

    for finding in findings {
        if current != Some(finding.file.as_path()) {
            current = Some(finding.file.as_path());
            let count = findings.iter().filter(|f| f.file == finding.file).count();
            out.push_str(&format!(
                "\n━━ {} ({} finding) ━━\n\n",
                finding.file.display(),
                count
            ));
        }
        let diag = FindingDiagnostic {
            source: NamedSource::new(
                finding.file.display().to_string(),
                finding.source.to_string(),
            ),
            finding: finding.clone(),
        };
        let _ = handler.render_report(&mut out, &diag);
        out.push('\n');
    }
    out
}
