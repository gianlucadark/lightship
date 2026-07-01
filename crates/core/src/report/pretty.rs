use crate::Analysis;
use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::report::{Glyphs, RenderOpts, find_meta, snippet, summary};
use crate::rules;
use owo_colors::OwoColorize;
use std::fmt::Write;

/// Quanti finding mostrare per file prima di comprimere il resto in una riga
/// `… +N altri`. `--verbose` rimuove il limite.
const MAX_PER_FILE: usize = 5;

/// Human-first terminal report: verdict, findings grouped by file, final summary.
pub fn render(analysis: &Analysis, opts: &RenderOpts) -> String {
    let g = opts.glyphs();
    let metas = rules::registry();
    let mut out = String::new();
    let errors = analysis.errors();
    let warnings = analysis.warnings();

    let _ = writeln!(
        out,
        "\n{}  {} {}",
        g.diamond.cyan().bold(),
        "LIGHTSHIP".cyan().bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).dimmed()
    );
    let _ = writeln!(out, "   {} {}", "Scanning".dimmed(), opts.dir.bold());

    let verdict = if errors > 0 {
        "CHECK FAILED".red().bold().to_string()
    } else {
        "CHECK PASSED".green().bold().to_string()
    };
    let counts = format!(
        "{} · {} · {}",
        count(errors, "error", "errors"),
        count(warnings, "warning", "warnings"),
        count(analysis.pages, "page", "pages"),
    );
    let _ = writeln!(out, "\n   {}  {}", verdict, counts.dimmed());

    if analysis.findings.is_empty() {
        let _ = writeln!(
            out,
            "\n   {} {}",
            g.check.green().bold(),
            "Your built HTML looks good.".green()
        );
    } else if !opts.quiet {
        out.push('\n');
        render_groups(&mut out, analysis, &metas, opts, &g);
    }

    out.push_str(&summary::render(analysis, opts));
    out
}

fn render_groups(
    out: &mut String,
    analysis: &Analysis,
    metas: &[RuleMeta],
    opts: &RenderOpts,
    g: &Glyphs,
) {
    let findings = &analysis.findings;
    let mut i = 0;
    while i < findings.len() {
        let file = &findings[i].file;
        let group: Vec<&Finding> = findings[i..]
            .iter()
            .take_while(|f| &f.file == file)
            .collect();
        i += group.len();
        render_file(out, &group, metas, opts, g);
    }
}

fn render_file(
    out: &mut String,
    group: &[&Finding],
    metas: &[RuleMeta],
    opts: &RenderOpts,
    g: &Glyphs,
) {
    let errors = group
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warnings = group.len() - errors;
    let path = group[0].file.display().to_string();

    let mut stats = Vec::new();
    if errors > 0 {
        stats.push(count(errors, "error", "errors").red().to_string());
    }
    if warnings > 0 {
        stats.push(count(warnings, "warning", "warnings").yellow().to_string());
    }
    // Punteggio della singola pagina, accanto alle statistiche.
    let (score, grade) = summary::score_from(errors, warnings);
    stats.push(color_grade(format!("{grade} {score}"), grade));

    let _ = writeln!(
        out,
        "{} {}  {}",
        g.tl.cyan().dimmed(),
        path.bold(),
        stats.join(" · ")
    );

    // Per non sommergere l'utente, mostriamo i primi MAX_PER_FILE finding (gli
    // errori vengono prima per via dell'ordinamento) e riassumiamo il resto.
    // `--verbose` mostra tutto.
    let shown = if opts.verbose {
        group.len()
    } else {
        group.len().min(MAX_PER_FILE)
    };
    for f in &group[..shown] {
        render_finding(out, f, metas, opts, g);
    }
    let hidden = group.len() - shown;
    if hidden > 0 {
        let _ = writeln!(
            out,
            "{}     {}",
            g.bar.cyan().dimmed(),
            format!("… +{hidden} more in this file (use --verbose to show all)").dimmed()
        );
    }
    let _ = writeln!(out, "{}\n", g.bl.cyan().dimmed());
}

fn render_finding(
    out: &mut String,
    f: &Finding,
    metas: &[RuleMeta],
    opts: &RenderOpts,
    g: &Glyphs,
) {
    let badge = match f.severity {
        Severity::Error => format!("{:<7}", "ERROR")
            .on_red()
            .white()
            .bold()
            .to_string(),
        Severity::Warn => format!("{:<7}", "WARNING")
            .on_yellow()
            .black()
            .bold()
            .to_string(),
    };
    let loc = match (f.line, f.column) {
        (Some(line), Some(column)) => format!(" · {line}:{column}"),
        _ => String::new(),
    };

    let _ = writeln!(out, "{}", g.bar.cyan().dimmed());
    let _ = writeln!(
        out,
        "{}  {}  {}{}",
        g.branch.cyan().dimmed(),
        badge,
        f.rule.cyan().bold(),
        loc.dimmed()
    );
    let _ = writeln!(out, "{}     {}", g.bar.cyan().dimmed(), f.message.bold());

    if f.span.is_some() {
        for line in snippet::render(f, 4, g, opts.width).lines() {
            let _ = writeln!(out, "{} {}", g.bar.cyan().dimmed(), line);
        }
    }

    if opts.suggestions
        && let Some(meta) = find_meta(metas, f.rule)
    {
        let _ = writeln!(
            out,
            "{}     {} {}",
            g.bar.cyan().dimmed(),
            "Fix".green().bold(),
            meta.help.dimmed()
        );
    }
}

/// Colora un'etichetta di voto (`A 95`) secondo il grado.
fn color_grade(text: String, grade: char) -> String {
    match grade {
        'A' | 'B' => text.green().to_string(),
        'C' | 'D' => text.yellow().to_string(),
        _ => text.red().to_string(),
    }
}

fn count(n: usize, singular: &str, plural: &str) -> String {
    format!("{n} {}", if n == 1 { singular } else { plural })
}
