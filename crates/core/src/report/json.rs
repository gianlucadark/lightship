use super::summary::{per_page, score};
use crate::Analysis;
use crate::rules;
use serde_json::json;

/// Versione dello schema JSON: incrementata quando cambia la forma dell'output,
/// così i consumer possono adattarsi. Solo campi aggiuntivi ⇒ retro-compatibile.
const SCHEMA_VERSION: u32 = 1;

/// Output JSON strutturato: riepilogo + lista finding arricchita con
/// posizione, snippet, categoria e suggerimento.
pub fn render(analysis: &Analysis) -> String {
    let metas = rules::registry();
    let (score, grade) = score(analysis);

    let findings: Vec<_> = analysis
        .findings
        .iter()
        .map(|f| {
            let meta = metas.iter().find(|m| m.id == f.rule);
            json!({
                "rule": f.rule,
                "severity": f.severity.label(),
                "category": meta.map(|m| m.category.label()),
                "message": f.message,
                "file": f.file.display().to_string().replace('\\', "/"),
                "line": f.line,
                "column": f.column,
                "snippet": f.snippet(),
                "help": meta.map(|m| m.help),
                "docs_url": meta.map(|m| m.docs_url),
            })
        })
        .collect();

    let pages: Vec<_> = per_page(analysis)
        .into_iter()
        .map(|p| {
            json!({
                "file": p.file.replace('\\', "/"),
                "errors": p.errors,
                "warnings": p.warnings,
                "score": p.score,
                "grade": p.grade.to_string(),
            })
        })
        .collect();

    let report = json!({
        "schema_version": SCHEMA_VERSION,
        "summary": {
            "pages": analysis.pages,
            "skipped": analysis.skipped,
            "baselined": analysis.baselined,
            "errors": analysis.errors(),
            "warnings": analysis.warnings(),
            "score": score,
            "grade": grade.to_string(),
            "elapsed_ms": analysis.elapsed.as_millis(),
        },
        "pages": pages,
        "findings": findings,
    });

    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
}
