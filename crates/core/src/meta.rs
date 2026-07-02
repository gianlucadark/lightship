use crate::finding::Severity;

/// Area a cui appartiene una regola. Serve a raggruppare l'output, filtrare con
/// `--only-category` e i preset, e taggare i finding in JSON/SARIF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Accessibility,
    Seo,
    Performance,
    Security,
    Correctness,
}

impl Category {
    /// Etichetta breve e stabile (usata in output e nei formati macchina).
    pub fn label(self) -> &'static str {
        match self {
            Category::Accessibility => "accessibility",
            Category::Seo => "seo",
            Category::Performance => "performance",
            Category::Security => "security",
            Category::Correctness => "correctness",
        }
    }

    /// Etichetta compatta per il pannello dei risultati.
    pub fn short_label(self) -> &'static str {
        match self {
            Category::Accessibility => "A11y",
            Category::Seo => "SEO",
            Category::Performance => "Perf",
            Category::Security => "Security",
            Category::Correctness => "Correctness",
        }
    }

    /// Parsa un nome di categoria (accetta anche alias comuni come `a11y`).
    pub fn parse(s: &str) -> Option<Category> {
        match s.trim().to_ascii_lowercase().as_str() {
            "accessibility" | "a11y" => Some(Category::Accessibility),
            "seo" => Some(Category::Seo),
            "performance" | "perf" => Some(Category::Performance),
            "security" | "sec" => Some(Category::Security),
            "correctness" | "html" => Some(Category::Correctness),
            _ => None,
        }
    }
}

/// Metadati statici di una regola: fonte unica per i suggerimenti del report,
/// il comando `rules`, il comando `explain` e la lista regole in SARIF.
#[derive(Debug, Clone, Copy)]
pub struct RuleMeta {
    pub id: &'static str,
    /// Gravità di default (può essere sovrascritta dalla config).
    pub severity: Severity,
    /// Area della regola (accessibilità, SEO, performance…).
    pub category: Category,
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
