use crate::Analysis;
use crate::finding::Severity;
use owo_colors::OwoColorize;
use std::collections::BTreeMap;
use std::fmt::Write;

/// Larghezza del pannello di riepilogo.
const WIDTH: usize = 52;
/// Larghezza massima delle barre del grafico "per regola".
const BAR_MAX: usize = 22;

/// Punteggio di qualità 0–100 e voto A–F.
///
/// Formula deliberatamente semplice e tarabile: ogni errore pesa 10, ogni
/// warning 3, con pavimento a 0.
pub fn score(analysis: &Analysis) -> (u8, char) {
    let penalty = analysis.errors() * 10 + analysis.warnings() * 3;
    let score = 100u32.saturating_sub(penalty as u32).min(100) as u8;
    let grade = match score {
        90..=100 => 'A',
        80..=89 => 'B',
        70..=79 => 'C',
        60..=69 => 'D',
        _ => 'F',
    };
    (score, grade)
}

/// Pannello di riepilogo: conteggi, grafico per regola, punteggio e verdetto.
pub fn render(analysis: &Analysis) -> String {
    let errors = analysis.errors();
    let warns = analysis.warnings();
    let (score, grade) = score(analysis);

    let mut out = String::new();
    out.push('\n');
    out.push_str(&rule_line(" Summary "));

    // Riga conteggi. I valori vanno colorati ma allineati sulla larghezza
    // *visibile* (i codici ANSI non contano): per questo `cell` fa il pad sulla
    // lunghezza del testo grezzo, non della stringa colorata.
    let pages_v = analysis.pages.to_string();
    let time_v = format!("{} ms", analysis.elapsed.as_millis());
    let err_v = format!("{errors} ✖");
    let warn_v = format!("{warns} ⚠");
    let _ = writeln!(
        out,
        " {} {} {} {}",
        format!("{:<9}", "Pages").dimmed(),
        cell(pages_v.bold().to_string(), pages_v.chars().count(), 12),
        format!("{:<9}", "Time").dimmed(),
        time_v.bold(),
    );
    let _ = writeln!(
        out,
        " {} {} {} {}",
        format!("{:<9}", "Errors").dimmed(),
        cell(err_v.red().bold().to_string(), err_v.chars().count(), 12),
        format!("{:<9}", "Warnings").dimmed(),
        warn_v.yellow().bold(),
    );

    // Grafico per regola.
    let by_rule = counts_by_rule(analysis);
    if !by_rule.is_empty() {
        let _ = writeln!(out, "\n {}", "By rule".dimmed());
        let max = by_rule.iter().map(|(_, &(e, w))| e + w).max().unwrap_or(1);
        // Ordina per conteggio desc, poi per id.
        let mut rows: Vec<(&&str, usize, usize)> =
            by_rule.iter().map(|(id, &(e, w))| (id, e, w)).collect();
        rows.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(b.0)));
        for (id, e, w) in rows {
            let total = e + w;
            let filled = (total * BAR_MAX).div_ceil(max.max(1));
            let bar = "█".repeat(filled);
            let bar = if e > 0 {
                bar.red().to_string()
            } else {
                bar.yellow().to_string()
            };
            let _ = writeln!(out, "   {:<16} {} {}", id, bar, total.bold());
        }
    }

    // Punteggio.
    let score_str = format!("{score}/100  ({grade})");
    let score_str = match grade {
        'A' | 'B' => score_str.green().bold().to_string(),
        'C' | 'D' => score_str.yellow().bold().to_string(),
        _ => score_str.red().bold().to_string(),
    };
    let _ = writeln!(
        out,
        "\n {} {}",
        format!("{:<9}", "Score").dimmed(),
        score_str
    );

    // Verdetto.
    out.push('\n');
    if errors > 0 {
        let _ = writeln!(
            out,
            " {}",
            format!(
                "✖  FAIL · {errors} {} to fix",
                plural(errors, "error", "errors")
            )
            .red()
            .bold()
        );
    } else if warns > 0 {
        let _ = writeln!(
            out,
            " {}",
            format!(
                "✔  PASS · no errors, {warns} {}",
                plural(warns, "warning", "warnings")
            )
            .green()
            .bold()
        );
    } else {
        let _ = writeln!(out, " {}", "✔  PASS · all clean".green().bold());
    }
    out.push_str(&rule_line(""));
    out
}

fn counts_by_rule(analysis: &Analysis) -> BTreeMap<&'static str, (usize, usize)> {
    let mut map: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    for f in &analysis.findings {
        let entry = map.entry(f.rule).or_default();
        match f.severity {
            Severity::Error => entry.0 += 1,
            Severity::Warn => entry.1 += 1,
        }
    }
    map
}

/// Una riga divisoria con titolo centrato, larga `WIDTH`.
fn rule_line(title: &str) -> String {
    let title_w = title.chars().count();
    let dashes = WIDTH.saturating_sub(title_w);
    let left = dashes / 2;
    let right = dashes - left;
    format!(
        "{}{}{}\n",
        "─".repeat(left).dimmed(),
        title.bold(),
        "─".repeat(right).dimmed()
    )
}

fn plural<'a>(n: usize, one: &'a str, many: &'a str) -> &'a str {
    if n == 1 { one } else { many }
}

/// Pad a destra di una stringa già colorata fino a `width` colonne *visibili*,
/// dove `visible` è la lunghezza del testo senza codici ANSI.
fn cell(colored: String, visible: usize, width: usize) -> String {
    if visible >= width {
        colored
    } else {
        format!("{colored}{}", " ".repeat(width - visible))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::Finding;
    use std::time::Duration;

    fn analysis(errors: usize, warns: usize) -> Analysis {
        let mut findings = Vec::new();
        for _ in 0..errors {
            findings.push(Finding::new("e", Severity::Error, "m", None));
        }
        for _ in 0..warns {
            findings.push(Finding::new("w", Severity::Warn, "m", None));
        }
        Analysis {
            pages: 1,
            findings,
            elapsed: Duration::from_millis(0),
        }
    }

    #[test]
    fn score_e_voto() {
        assert_eq!(score(&analysis(0, 0)), (100, 'A'));
        // 1 errore (10) + 3 warning (9) = 19 → 81 → B
        assert_eq!(score(&analysis(1, 3)), (81, 'B'));
        // tanti errori → pavimento a 0
        assert_eq!(score(&analysis(20, 0)), (0, 'F'));
    }
}
