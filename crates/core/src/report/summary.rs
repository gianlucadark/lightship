use crate::Analysis;
use crate::finding::Severity;
use owo_colors::OwoColorize;
use std::collections::BTreeMap;
use std::fmt::Write;

const WIDTH: usize = 62;

/// Quality score used by the terminal and JSON reports.
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

/// Compact dashboard placed after the detailed findings.
pub fn render(analysis: &Analysis) -> String {
    let errors = analysis.errors();
    let warnings = analysis.warnings();
    let (score, grade) = score(analysis);
    let mut out = String::new();

    out.push_str(&top_border(" Results "));

    let verdict = if errors > 0 {
        "FAIL".on_red().white().bold().to_string()
    } else {
        "PASS".on_green().black().bold().to_string()
    };
    let error_text = format!("{errors} {}", plural(errors, "error", "errors"));
    let warning_text = format!("{warnings} {}", plural(warnings, "warning", "warnings"));
    let _ = writeln!(
        out,
        "{}  {}  {}  {}",
        "│".dimmed(),
        verdict,
        error_text.red().bold(),
        warning_text.yellow().bold()
    );

    let health = health_bar(score);
    let score_text = format!("{score}/100 · grade {grade}");
    let _ = writeln!(
        out,
        "{}  {}  {}  {}",
        "│".dimmed(),
        "Health".dimmed(),
        health,
        color_score(score_text, grade)
    );
    let _ = writeln!(
        out,
        "{}  {} {}  {} {}",
        "│".dimmed(),
        "Pages".dimmed(),
        analysis.pages.to_string().bold(),
        "Time".dimmed(),
        format!("{} ms", analysis.elapsed.as_millis()).bold()
    );

    let rows = sorted_rule_counts(analysis);
    if !rows.is_empty() {
        out.push_str(&middle_border(" Issues by rule "));
        for (id, errors, warnings) in &rows {
            let marker = if *errors > 0 {
                "●".red().to_string()
            } else {
                "●".yellow().to_string()
            };
            let total = errors + warnings;
            let _ = writeln!(
                out,
                "{}  {} {:<22} {}",
                "│".dimmed(),
                marker,
                id,
                total.to_string().bold()
            );
        }

        // Guida l'utente medio: la regola più frequente è il punto di partenza.
        if let Some((id, errors, warnings)) = rows.first() {
            let total = errors + warnings;
            let _ = writeln!(out, "{}", "│".dimmed());
            let _ = writeln!(
                out,
                "{}  {} {} {}",
                "│".dimmed(),
                "Start here".green().bold(),
                format!("{id} ({total})").bold(),
                format!("→ lightship explain {id}").dimmed(),
            );
        }
    }

    out.push_str(&bottom_border());
    out
}

fn sorted_rule_counts(analysis: &Analysis) -> Vec<(&'static str, usize, usize)> {
    let mut counts: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    for finding in &analysis.findings {
        let entry = counts.entry(finding.rule).or_default();
        match finding.severity {
            Severity::Error => entry.0 += 1,
            Severity::Warn => entry.1 += 1,
        }
    }
    let mut rows: Vec<_> = counts
        .into_iter()
        .map(|(id, (errors, warnings))| (id, errors, warnings))
        .collect();
    rows.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| (b.1 + b.2).cmp(&(a.1 + a.2)))
            .then_with(|| a.0.cmp(b.0))
    });
    rows
}

fn health_bar(score: u8) -> String {
    const CELLS: usize = 10;
    let filled = ((score as usize * CELLS) + 99) / 100;
    let raw = format!("{}{}", "■".repeat(filled), "·".repeat(CELLS - filled));
    match score {
        90..=100 => raw.green().to_string(),
        60..=89 => raw.yellow().to_string(),
        _ => raw.red().to_string(),
    }
}

fn color_score(text: String, grade: char) -> String {
    match grade {
        'A' | 'B' => text.green().bold().to_string(),
        'C' | 'D' => text.yellow().bold().to_string(),
        _ => text.red().bold().to_string(),
    }
}

fn top_border(title: &str) -> String {
    titled_border("╭─", title)
}

fn middle_border(title: &str) -> String {
    titled_border("├─", title)
}

fn titled_border(left: &str, title: &str) -> String {
    let remaining = WIDTH.saturating_sub(title.chars().count());
    format!(
        "{}{}{}\n",
        left.dimmed(),
        title.bold(),
        "─".repeat(remaining).dimmed(),
    )
}

fn bottom_border() -> String {
    format!("{}{}\n", "╰".dimmed(), "─".repeat(WIDTH).dimmed())
}

fn plural<'a>(n: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if n == 1 { singular } else { plural }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::Finding;
    use std::time::Duration;

    fn analysis(errors: usize, warnings: usize) -> Analysis {
        let mut findings = Vec::new();
        for _ in 0..errors {
            findings.push(Finding::new("e", Severity::Error, "m", None));
        }
        for _ in 0..warnings {
            findings.push(Finding::new("w", Severity::Warn, "m", None));
        }
        Analysis {
            pages: 1,
            findings,
            elapsed: Duration::from_millis(0),
        }
    }

    #[test]
    fn score_and_grade() {
        assert_eq!(score(&analysis(0, 0)), (100, 'A'));
        assert_eq!(score(&analysis(1, 3)), (81, 'B'));
        assert_eq!(score(&analysis(20, 0)), (0, 'F'));
    }

    #[test]
    fn health_bar_has_ten_cells() {
        let bar = health_bar(45);
        assert!(bar.contains("■■■■■·····"));
    }
}
