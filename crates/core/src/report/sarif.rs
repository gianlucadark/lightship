use crate::Analysis;
use crate::finding::Severity;
use crate::rules;
use serde_json::json;

/// Output SARIF 2.1.0 minimale per GitHub code scanning.
///
/// `tool.driver.rules` elenca tutte le regole con descrizione/help; ogni
/// `result` riferisce la regola e localizza il problema (file + riga/colonna).
pub fn render(analysis: &Analysis) -> String {
    let metas = rules::registry();

    let rules_json: Vec<_> = metas
        .iter()
        .map(|m| {
            json!({
                "id": m.id,
                "name": m.id,
                "shortDescription": { "text": m.summary },
                "fullDescription": { "text": m.help },
                "helpUri": m.docs_url,
                "defaultConfiguration": { "level": sarif_level(m.severity) },
            })
        })
        .collect();

    let results: Vec<_> = analysis
        .findings
        .iter()
        .map(|f| {
            let uri = f.file.display().to_string().replace('\\', "/");
            json!({
                "ruleId": f.rule,
                "level": sarif_level(f.severity),
                "message": { "text": f.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": uri },
                        "region": {
                            "startLine": f.line.unwrap_or(1),
                            "startColumn": f.column.unwrap_or(1),
                        }
                    }
                }],
            })
        })
        .collect();

    let sarif = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "Lightship",
                    "informationUri": "https://github.com/gianlucadark/lightship",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rules_json,
                }
            },
            "results": results,
        }],
    });

    serde_json::to_string_pretty(&sarif).unwrap_or_else(|_| "{}".to_string())
}

fn sarif_level(sev: Severity) -> &'static str {
    match sev {
        Severity::Error => "error",
        Severity::Warn => "warning",
    }
}
