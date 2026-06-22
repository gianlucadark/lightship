use std::path::PathBuf;
use std::sync::Arc;

/// Gravità di un finding. In Fase 0 usiamo solo Error, ma il tipo è già pronto
/// per regole "warn" future.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warn,
}

/// Un problema trovato da una regola su una pagina.
///
/// `file` e `source` non sono noti a `Rule::check` (che riceve solo il DOM e il
/// sorgente): li riempie l'orchestratore dopo aver eseguito la regola.
///
/// `span` è l'intervallo `(offset, len)` in byte del tag di apertura
/// dell'elemento incriminato, riferito a `source`: così possiamo mostrare lo
/// **snippet di codice reale** (estratto dal file) con numeri di riga corretti.
/// È `None` per i finding "di documento" (es. `<meta>` mancante) che non hanno
/// un elemento a cui puntare.
#[derive(Debug, Clone)]
pub struct Finding {
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
    pub file: PathBuf,
    pub span: Option<(usize, usize)>,
    pub source: Arc<str>,
}

impl Finding {
    /// Crea un finding senza `file`/`source` (li imposta poi l'orchestratore).
    pub fn new(
        rule: &'static str,
        severity: Severity,
        message: impl Into<String>,
        span: Option<(usize, usize)>,
    ) -> Self {
        Finding {
            rule,
            severity,
            message: message.into(),
            file: PathBuf::new(),
            span,
            source: Arc::from(""),
        }
    }

    /// Lo snippet di codice reale estratto dal sorgente del file, se il finding
    /// ha uno span. Per i finding "di documento" ritorna `None`.
    pub fn snippet(&self) -> Option<&str> {
        self.span
            .and_then(|(off, len)| self.source.get(off..off + len))
    }
}
