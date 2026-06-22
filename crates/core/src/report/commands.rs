use crate::finding::Severity;
use crate::meta::RuleMeta;
use owo_colors::OwoColorize;
use std::fmt::Write;

/// Tabella di tutte le regole (comando `rules`): id, gravità, descrizione.
pub fn rules_table(metas: &[RuleMeta]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\n🛳  {} regole\n", format!("{}", metas.len()).bold());
    let _ = writeln!(
        out,
        " {} {} {}",
        format!("{:<18}", "REGOLA").dimmed(),
        format!("{:<8}", "GRAVITÀ").dimmed(),
        "DESCRIZIONE".dimmed(),
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
        "Dettagli su una regola: lightship explain <regola>".dimmed()
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
    let _ = writeln!(out, "\n{}", "Come correggere".bold());
    let _ = writeln!(out, "  💡 {}", m.help);
    let _ = writeln!(out, "\n{}", "Esempi".bold());
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
