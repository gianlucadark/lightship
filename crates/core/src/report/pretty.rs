use crate::Analysis;
use crate::finding::{Finding, Severity};
use crate::meta::RuleMeta;
use crate::report::{RenderOpts, find_meta, snippet, summary};
use crate::rules;
use owo_colors::OwoColorize;
use std::fmt::Write;

/// Dashboard a colori: banner, finding raggruppati per file e pannello finale.
pub fn render(analysis: &Analysis, opts: &RenderOpts) -> String {
    let metas = rules::registry();
    let mut out = String::new();

    // Banner.
    let _ = writeln!(
        out,
        "\n🛳  {} {}",
        "Lightship".cyan().bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).dimmed()
    );
    let _ = writeln!(out, "{}\n", format!("   analyzing {}", opts.dir).dimmed());

    if analysis.findings.is_empty() {
        let _ = writeln!(out, "  {}  No issues found.", "✔".green().bold());
    } else if !opts.quiet {
        render_groups(&mut out, analysis, &metas, opts);
    }

    out.push_str(&summary::render(analysis));
    out
}

/// Raggruppa i finding (già ordinati per file) e renderizza ogni gruppo.
fn render_groups(out: &mut String, analysis: &Analysis, metas: &[RuleMeta], opts: &RenderOpts) {
    let findings = &analysis.findings;
    let mut i = 0;
    while i < findings.len() {
        let file = &findings[i].file;
        let group: Vec<&Finding> = findings[i..]
            .iter()
            .take_while(|f| &f.file == file)
            .collect();
        i += group.len();
        render_file(out, &group, metas, opts);
    }
}

fn render_file(out: &mut String, group: &[&Finding], metas: &[RuleMeta], opts: &RenderOpts) {
    let errors = group
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warns = group.len() - errors;
    let path = group[0].file.display().to_string();

    let err_badge = if errors > 0 {
        format!("✖{errors}").red().to_string()
    } else {
        format!("✖{errors}").dimmed().to_string()
    };
    let warn_badge = if warns > 0 {
        format!("⚠{warns}").yellow().to_string()
    } else {
        format!("⚠{warns}").dimmed().to_string()
    };

    let issue_word = if group.len() == 1 { "issue" } else { "issues" };
    let _ = writeln!(
        out,
        "{} {}  {}  {} {}",
        "❯".cyan().bold(),
        path.bold(),
        format!("· {} {issue_word}", group.len()).dimmed(),
        err_badge,
        warn_badge,
    );

    for f in group {
        render_finding(out, f, metas, opts);
    }
}

fn render_finding(out: &mut String, f: &Finding, metas: &[RuleMeta], opts: &RenderOpts) {
    let badge = match f.severity {
        Severity::Error => "✖".red().bold().to_string(),
        Severity::Warn => "⚠".yellow().bold().to_string(),
    };
    let loc = match (f.line, f.column) {
        (Some(l), Some(c)) => format!("  {}", format!("L{l}:C{c}").dimmed()),
        _ => String::new(),
    };

    let _ = writeln!(out, "  {} {}{}", badge, f.rule.bold(), loc);
    let _ = writeln!(out, "    {}", f.message);

    if f.span.is_some() {
        out.push_str(&snippet::render(f, 4));
    }

    if opts.suggestions {
        if let Some(meta) = find_meta(metas, f.rule) {
            let _ = writeln!(out, "    💡 {}", meta.help.italic().dimmed());
        }
    }
    out.push('\n');
}
