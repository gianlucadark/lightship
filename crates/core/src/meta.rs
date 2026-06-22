use crate::finding::Severity;

/// Metadati statici di una regola: fonte unica per i suggerimenti del report,
/// il comando `rules`, il comando `explain` e la lista regole in SARIF.
#[derive(Debug, Clone, Copy)]
pub struct RuleMeta {
    pub id: &'static str,
    /// Gravità di default (può essere sovrascritta dalla config).
    pub severity: Severity,
    /// Una riga: cosa controlla la regola.
    pub summary: &'static str,
    /// 💡 Come correggere il problema (il "suggerimento" mostrato nel report).
    pub help: &'static str,
    /// Esempio di codice che fa scattare la regola.
    pub example_bad: &'static str,
    /// Esempio di codice corretto.
    pub example_good: &'static str,
    /// Link alla documentazione/standard di riferimento.
    pub docs_url: &'static str,
}
