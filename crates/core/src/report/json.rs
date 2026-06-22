use super::summary::score;
use crate::Analysis;
use crate::rules;
use serde_json::json;

/// Output JSON strutturato: riepilogo + lista finding arricchita con
/// posizione, snippet e suggerimento.
pub fn render(analysis: &Analysis) -> String {
    let metas = rules::registry();
    let (score, grade) = score(analysis);

    let findings: Vec<_> = analysis
        .findings
        .iter()
        .map(|f| {
            let help = metas.iter().find(|m| m.id == f.rule).map(|m| m.help);
            json!({
                "rule": f.rule,
                "severity": f.severity.label(),
                "message": f.message,
                "file": f.file.display().to_string().replace('\\', "/"),
                "line": f.line,
                "column": f.column,
                "snippet": f.snippet(),
                "help": help,
            })
        })
        .collect();

    let report = json!({
        "summary": {
            "pages": analysis.pages,
            "errors": analysis.errors(),
            "warnings": analysis.warnings(),
            "score": score,
            "grade": grade.to_string(),
            "elapsed_ms": analysis.elapsed.as_millis(),
        },
        "findings": findings,
    });

    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
}
