use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;

/// Gravità di un finding.
///
/// L'ordine (`Error` < `Warn`) è scelto perché serve a ordinare i finding con
/// gli errori prima; serializza in minuscolo (`"error"`/`"warn"`) per JSON/SARIF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warn,
}

impl Severity {
    /// Etichetta breve usata nei messaggi e nei formati testuali.
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warn => "warn",
        }
    }
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
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
    pub file: PathBuf,
    /// Riga/colonna 1-based, calcolate dallo span: comode per JSON/SARIF/GitHub.
    #[serde(rename = "line")]
    pub line: Option<usize>,
    #[serde(rename = "column")]
    pub column: Option<usize>,
    /// Lo span grezzo e il sorgente servono solo al rendering interno.
    #[serde(skip)]
    pub span: Option<(usize, usize)>,
    #[serde(skip)]
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
            line: None,
            column: None,
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

    /// Riga e colonna 1-based dell'inizio dello span, calcolate da `source`.
    /// `None` per i finding "di documento" (senza span).
    pub fn location(&self) -> Option<(usize, usize)> {
        let (offset, _) = self.span?;
        let prefix = self.source.get(..offset)?;
        let line = prefix.bytes().filter(|&b| b == b'\n').count() + 1;
        let col = prefix.len() - prefix.rfind('\n').map_or(0, |i| i + 1) + 1;
        Some((line, col))
    }

    /// Aggancia file e sorgente (chiamato dall'orchestratore) e calcola la
    /// posizione riga/colonna così che resti nel finding anche dopo il drop
    /// del DOM.
    pub fn attach(&mut self, file: PathBuf, source: Arc<str>) {
        self.file = file;
        self.source = source;
        if let Some((line, col)) = self.location() {
            self.line = Some(line);
            self.column = Some(col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn location_riga_e_colonna() {
        let src = "riga1\n  <img>\n";
        // `<img>` inizia all'offset del `<` sulla seconda riga (colonna 3).
        let offset = src.find('<').unwrap();
        let mut f = Finding::new("x", Severity::Error, "m", Some((offset, 5)));
        f.attach(PathBuf::from("a.html"), Arc::from(src));
        assert_eq!(f.location(), Some((2, 3)));
        assert_eq!((f.line, f.column), (Some(2), Some(3)));
        assert_eq!(f.snippet(), Some("<img>"));
    }

    #[test]
    fn finding_di_documento_senza_posizione() {
        let mut f = Finding::new("x", Severity::Warn, "m", None);
        f.attach(PathBuf::from("a.html"), Arc::from("<html></html>"));
        assert_eq!(f.location(), None);
        assert_eq!(f.line, None);
    }
}
