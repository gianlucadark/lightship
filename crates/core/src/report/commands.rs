use crate::finding::Severity;
use crate::meta::{Category, RuleMeta};
use owo_colors::OwoColorize;
use std::fmt::Write;

/// Ordine di presentazione delle categorie nella tabella `rules`.
const CATEGORY_ORDER: &[Category] = &[
    Category::Accessibility,
    Category::Seo,
    Category::Performance,
    Category::Security,
    Category::Correctness,
];

/// Tabella di tutte le regole (comando `rules`), raggruppate per categoria.
pub fn rules_table(metas: &[RuleMeta]) -> String {
    let mut out = String::new();
    // Larghezza della colonna RULE: il più lungo fra gli id e l'intestazione,
    // così la tabella resta allineata anche con id lunghi (es. quelli `-length`).
    let id_w = metas
        .iter()
        .map(|m| m.id.len())
        .chain(std::iter::once("RULE".len()))
        .max()
        .unwrap_or(4);
    let _ = writeln!(out, "\n🛳  {} rules\n", format!("{}", metas.len()).bold());

    for &category in CATEGORY_ORDER {
        let rules: Vec<&RuleMeta> = metas.iter().filter(|m| m.category == category).collect();
        if rules.is_empty() {
            continue;
        }
        let _ = writeln!(out, "{}", category_title(category).bold());
        for m in rules {
            let _ = writeln!(
                out,
                "  {:<id_w$} {} {}",
                m.id.bold(),
                severity_badge(m.severity),
                m.summary,
            );
        }
        out.push('\n');
    }

    let _ = writeln!(
        out,
        "{}",
        "Details for a rule: lightship explain <rule>".dimmed()
    );
    out
}

/// Titolo leggibile di una categoria per le intestazioni della tabella.
fn category_title(c: Category) -> &'static str {
    match c {
        Category::Accessibility => "Accessibility",
        Category::Seo => "SEO",
        Category::Performance => "Performance",
        Category::Security => "Security",
        Category::Correctness => "Correctness",
    }
}

/// Scheda dettagliata di una regola (comando `explain`).
pub fn explain(m: &RuleMeta) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n{}  {}  {}",
        m.id.cyan().bold(),
        severity_badge(m.severity),
        category_title(m.category).dimmed()
    );
    let _ = writeln!(out, "\n{}", m.summary);
    let _ = writeln!(out, "\n{}", "How to fix".bold());
    let _ = writeln!(out, "  💡 {}", m.help);
    let _ = writeln!(out, "\n{}", "Examples".bold());
    let _ = writeln!(out, "  {} {}", "✖".red(), m.example_bad.red());
    let _ = writeln!(out, "  {} {}", "✔".green(), m.example_good.green());
    let _ = writeln!(
        out,
        "\n{} {}",
        "Docs:".dimmed(),
        m.docs_url.dimmed().underline()
    );
    out
}

fn severity_badge(sev: Severity) -> String {
    match sev {
        Severity::Error => format!("{:<8}", "✖ error").red().bold().to_string(),
        Severity::Warn => format!("{:<8}", "⚠ warn").yellow().bold().to_string(),
    }
}
