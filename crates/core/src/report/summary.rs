use crate::Analysis;
use crate::finding::Severity;
use crate::meta::Category;
use crate::report::{Glyphs, RenderOpts};
use crate::rules;
use owo_colors::OwoColorize;
use std::collections::BTreeMap;
use std::fmt::Write;

/// Larghezza massima del pannello di riepilogo; entro questa si adatta alla
/// larghezza del terminale.
const MAX_WIDTH: usize = 62;

/// Quality score used by the terminal and JSON reports.
pub fn score(analysis: &Analysis) -> (u8, char) {
    score_from(analysis.errors(), analysis.warnings())
}

/// Punteggio 0–100 e voto A–F da un conteggio di errori/warning. Estratto così
/// da riusarlo anche per il punteggio **per pagina**.
pub fn score_from(errors: usize, warnings: usize) -> (u8, char) {
    let penalty = errors * 10 + warnings * 3;
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

/// Statistiche e punteggio di una singola pagina.
pub struct PageStats {
    pub file: String,
    pub errors: usize,
    pub warnings: usize,
    pub score: u8,
    pub grade: char,
}

/// Statistiche per pagina (un elemento per file con almeno un finding),
/// ordinate per nome file. Usate dal report JSON e dalla dashboard.
pub fn per_page(analysis: &Analysis) -> Vec<PageStats> {
    let mut map: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for f in &analysis.findings {
        let entry = map.entry(f.file.display().to_string()).or_default();
        match f.severity {
            Severity::Error => entry.0 += 1,
            Severity::Warn => entry.1 += 1,
        }
    }
    map.into_iter()
        .map(|(file, (errors, warnings))| {
            let (score, grade) = score_from(errors, warnings);
            PageStats {
                file,
                errors,
                warnings,
                score,
                grade,
            }
        })
        .collect()
}

/// Compact dashboard placed after the detailed findings.
pub fn render(analysis: &Analysis, opts: &RenderOpts) -> String {
    let g = opts.glyphs();
    let width = opts.width.min(MAX_WIDTH);
    let errors = analysis.errors();
    let warnings = analysis.warnings();
    let (score, grade) = score(analysis);
    let mut out = String::new();

    out.push_str(&top_border(&g, width, " Results "));

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
        g.bar.dimmed(),
        verdict,
        error_text.red().bold(),
        warning_text.yellow().bold()
    );

    let health = health_bar(score, &g);
    let score_text = format!("{score}/100 · grade {grade}");
    let _ = writeln!(
        out,
        "{}  {}  {}  {}",
        g.bar.dimmed(),
        "Health".dimmed(),
        health,
        color_score(score_text, grade)
    );
    let _ = writeln!(
        out,
        "{}  {} {}  {} {}",
        g.bar.dimmed(),
        "Pages".dimmed(),
        analysis.pages.to_string().bold(),
        "Time".dimmed(),
        format!("{} ms", analysis.elapsed.as_millis()).bold()
    );
    // Ripartizione dei finding per categoria: dà il colpo d'occhio su *dove*
    // sta il problema (a11y? SEO?) prima della lista per regola.
    if !analysis.findings.is_empty() {
        let counts = category_counts(analysis);
        let parts: Vec<String> = CATEGORIES
            .iter()
            .zip(counts)
            .map(|(cat, n)| {
                let text = format!("{} {n}", cat.short_label());
                if n > 0 {
                    text.bold().to_string()
                } else {
                    text.dimmed().to_string()
                }
            })
            .collect();
        let _ = writeln!(
            out,
            "{}  {} {}",
            g.bar.dimmed(),
            "Issues".dimmed(),
            parts.join(&format!(" {} ", "·".dimmed()))
        );
    }

    if analysis.skipped > 0 {
        let _ = writeln!(
            out,
            "{}  {} {}",
            g.bar.dimmed(),
            "Skipped".dimmed(),
            format!(
                "{} {} (unreadable, unparseable or too large)",
                analysis.skipped,
                plural(analysis.skipped, "file", "files")
            )
            .dimmed()
        );
    }
    if analysis.baselined > 0 {
        let _ = writeln!(
            out,
            "{}  {} {}",
            g.bar.dimmed(),
            "Baseline".dimmed(),
            format!(
                "{} known {} suppressed",
                analysis.baselined,
                plural(analysis.baselined, "issue", "issues")
            )
            .dimmed()
        );
    }

    let rows = sorted_rule_counts(analysis);
    if !rows.is_empty() {
        out.push_str(&middle_border(&g, width, " Issues by rule "));
        for (id, errors, warnings) in &rows {
            let marker = if *errors > 0 {
                g.dot.red().to_string()
            } else {
                g.dot.yellow().to_string()
            };
            let total = errors + warnings;
            let _ = writeln!(
                out,
                "{}  {} {:<22} {}",
                g.bar.dimmed(),
                marker,
                id,
                total.to_string().bold()
            );
        }

        // Guida l'utente medio: la regola più frequente è il punto di partenza.
        if let Some((id, errors, warnings)) = rows.first() {
            let total = errors + warnings;
            let _ = writeln!(out, "{}", g.bar.dimmed());
            let _ = writeln!(
                out,
                "{}  {} {} {}",
                g.bar.dimmed(),
                "Start here".green().bold(),
                format!("{id} ({total})").bold(),
                format!("→ lightship explain {id}").dimmed(),
            );
        }
    }

    out.push_str(&bottom_border(&g, width));
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

/// Ordine fisso delle categorie nella riga `Issues` del pannello.
const CATEGORIES: [Category; 5] = [
    Category::Accessibility,
    Category::Seo,
    Category::Performance,
    Category::Security,
    Category::Correctness,
];

/// Conta i finding per categoria, nell'ordine di [`CATEGORIES`].
fn category_counts(analysis: &Analysis) -> [usize; 5] {
    let mut counts = [0usize; 5];
    for f in &analysis.findings {
        if let Some(m) = rules::meta(f.rule)
            && let Some(i) = CATEGORIES.iter().position(|&c| c == m.category)
        {
            counts[i] += 1;
        }
    }
    counts
}

fn health_bar(score: u8, g: &Glyphs) -> String {
    const CELLS: usize = 10;
    // Arrotondamento round-half-up: con `div_ceil` un 91 mostrava la barra
    // piena come un 100, in disaccordo con voto e punteggio.
    let filled = (score as usize * CELLS + 50) / 100;
    let raw = format!(
        "{}{}",
        g.cell_full.repeat(filled),
        g.cell_empty.repeat(CELLS - filled)
    );
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

fn top_border(g: &Glyphs, width: usize, title: &str) -> String {
    titled_border(g.panel_tl, g, width, title)
}

fn middle_border(g: &Glyphs, width: usize, title: &str) -> String {
    titled_border(g.panel_branch, g, width, title)
}

fn titled_border(left: &str, g: &Glyphs, width: usize, title: &str) -> String {
    let remaining = width.saturating_sub(title.chars().count());
    format!(
        "{}{}{}\n",
        left.dimmed(),
        title.bold(),
        g.hline.repeat(remaining).dimmed(),
    )
}

fn bottom_border(g: &Glyphs, width: usize) -> String {
    format!(
        "{}{}\n",
        g.panel_bl.dimmed(),
        g.hline.repeat(width).dimmed()
    )
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
            skipped: 0,
            baselined: 0,
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
        let bar = health_bar(45, &Glyphs::new(false));
        assert!(bar.contains("■■■■■·····"));
    }

    #[test]
    fn health_bar_ascii() {
        let bar = health_bar(45, &Glyphs::new(true));
        assert!(bar.contains("#####....."));
    }

    #[test]
    fn health_bar_piena_solo_vicino_al_100() {
        // Con div_ceil un 91 mostrava 10 celle come un 100.
        let g = Glyphs::new(true);
        assert!(health_bar(91, &g).contains("#########."));
        assert!(health_bar(95, &g).contains("##########"));
        assert!(health_bar(100, &g).contains("##########"));
    }

    #[test]
    fn riga_categorie_nel_pannello() {
        let opts = RenderOpts {
            suggestions: true,
            verbose: false,
            quiet: false,
            dir: ".".into(),
            ascii: true,
            width: 80,
        };
        let out = render(&analysis(1, 2), &opts);
        assert!(out.contains("Issues"), "{out}");
    }
}
