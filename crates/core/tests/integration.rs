use lightship_core::{Finding, Severity, analyze};

/// Cartella delle fixture, risolta in modo robusto rispetto alla CWD dei test.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn count_rule(findings: &[Finding], rule: &str) -> usize {
    findings.iter().filter(|f| f.rule == rule).count()
}

#[test]
fn conta_i_finding_attesi_sulle_fixture() {
    let analysis = analyze(FIXTURES);

    // 10 pagine, ognuna isola una singola violazione (tranne clean.html).
    assert_eq!(analysis.pages, 10, "dovrebbe trovare 10 file .html");

    // Una violazione per ogni regola.
    for rule in [
        "img-alt",
        "html-lang",
        "title-present",
        "duplicate-id",
        "meta-charset",
        "meta-viewport",
        "meta-description",
        "img-dimensions",
        "a-no-text",
    ] {
        assert_eq!(count_rule(&analysis.findings, rule), 1, "regola {rule}");
    }

    let errors = analysis
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warns = analysis
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warn)
        .count();

    // Error: img-alt, html-lang, title-present, duplicate-id.
    assert_eq!(errors, 4);
    // Warn: meta-charset, meta-viewport, meta-description, img-dimensions, a-no-text.
    assert_eq!(warns, 5);
    assert_eq!(analysis.findings.len(), 9);
}

#[test]
fn ogni_finding_ha_file_e_sorgente() {
    let analysis = analyze(FIXTURES);
    assert!(!analysis.findings.is_empty());
    for f in &analysis.findings {
        assert!(!f.file.as_os_str().is_empty(), "il file va impostato");
        assert!(!f.source.is_empty(), "il sorgente va impostato");
        // i finding con uno span devono esporre lo snippet di codice reale,
        // estratto davvero dal sorgente del file.
        if let Some(snippet) = f.snippet() {
            assert!(!snippet.is_empty(), "lo snippet non deve essere vuoto");
            assert!(
                f.source.contains(snippet),
                "lo snippet deve essere codice reale preso dal file"
            );
        }
    }
}

#[test]
fn clean_html_non_produce_finding() {
    let analysis = analyze(FIXTURES);
    let clean = analysis
        .findings
        .iter()
        .filter(|f| f.file.file_name().is_some_and(|n| n == "clean.html"))
        .count();
    assert_eq!(clean, 0, "clean.html deve essere pulito");
}
