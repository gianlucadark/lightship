use crate::finding::Severity;
use crate::meta::RuleMeta;
use owo_colors::OwoColorize;
use std::fmt::Write;

/// Tabella di tutte le regole (comando `rules`): id, gravità, descrizione.
pub fn rules_table(metas: &[RuleMeta]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\n🛳  {} rules\n", format!("{}", metas.len()).bold());
    let _ = writeln!(
        out,
        " {} {} {}",
        format!("{:<18}", "RULE").dimmed(),
        format!("{:<8}", "SEVERITY").dimmed(),
        "DESCRIPTION".dimmed(),
    );
    for m in metas {
        let _ = writeln!(
            out,
            " {:<18} {} {}",
            m.id.bold(),
            severity_badge(m.severity),
            m.summary,
        );
    }
    let _ = writeln!(
        out,
        "\n{}",
        "Details for a rule: lightship explain <rule>".dimmed()
    );
    out
}

/// Scheda dettagliata di una regola (comando `explain`).
pub fn explain(m: &RuleMeta) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n{}  {}",
        m.id.cyan().bold(),
        severity_badge(m.severity)
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
