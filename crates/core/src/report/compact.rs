use crate::Analysis;
use crate::finding::{Finding, Severity};
use crate::report::{RenderOpts, find_meta};
use crate::rules;
use owo_colors::OwoColorize;
use std::fmt::Write;

/// Output compatto: una riga per finding, raggruppati per file. Denso e
/// facile da grep-pare nei log di CI.
pub fn render(analysis: &Analysis, opts: &RenderOpts) -> String {
    let metas = rules::registry();
    let mut out = String::new();

    if !opts.quiet {
        let findings = &analysis.findings;
        let mut i = 0;
        while i < findings.len() {
            let file = &findings[i].file;
            let group: Vec<&Finding> = findings[i..]
                .iter()
                .take_while(|f| &f.file == file)
                .collect();
            i += group.len();

            let _ = writeln!(out, "{}", file.display().to_string().underline());
            for f in &group {
                render_finding(&mut out, f, &metas, opts);
            }
        }
        if !findings.is_empty() {
            out.push('\n');
        }
    }

    // Riga di riepilogo finale.
    let (errors, warns) = (analysis.errors(), analysis.warnings());
    let verdict = if errors > 0 {
        format!("✖ {errors} error, {warns} warn")
            .red()
            .bold()
            .to_string()
    } else {
        format!("✔ {errors} error, {warns} warn")
            .green()
            .bold()
            .to_string()
    };
    let _ = writeln!(
        out,
        "{} · {} pages · {} ms",
        verdict,
        analysis.pages,
        analysis.elapsed.as_millis()
    );
    out
}

fn render_finding(
    out: &mut String,
    f: &Finding,
    metas: &[crate::meta::RuleMeta],
    opts: &RenderOpts,
) {
    let badge = match f.severity {
        Severity::Error => "✖".red().to_string(),
        Severity::Warn => "⚠".yellow().to_string(),
    };
    let loc = match (f.line, f.column) {
        (Some(l), Some(c)) => format!("{l}:{c}"),
        _ => "—".to_string(),
    };
    let _ = writeln!(
        out,
        "  {} {:<16} {} {}",
        badge,
        f.rule,
        format!("{loc:<7}").dimmed(),
        f.message
    );
    if opts.suggestions {
        if let Some(meta) = find_meta(metas, f.rule) {
            let _ = writeln!(out, "      💡 {}", meta.help.dimmed());
        }
    }
}
