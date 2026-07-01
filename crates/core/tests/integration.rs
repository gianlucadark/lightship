use lightship_core::{
    BASELINE_FILE, Finding, Fix, Options, Severity, analyze, analyze_opts, apply_fixes,
    build_baseline, collect_fixes,
};

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

/// Un frammento (niente `<html>`/`<head>`/doctype) non deve far scattare le
/// regole "di documento" (title/charset/viewport/single-h1...), ma deve comunque
/// far scattare quelle sugli elementi (img senza alt). Con `include_fragments`
/// tornano tutte attive.
#[test]
fn i_frammenti_saltano_le_regole_di_documento() {
    let base = std::env::temp_dir().join(format!("lightship-fragment-{}", std::process::id()));
    std::fs::create_dir_all(&base).unwrap();
    // Frammento tipico di un componente/partial: nessun <html>/<head>.
    std::fs::write(
        base.join("card.html"),
        r#"<div class="card"><img src="a.png"></div>"#,
    )
    .unwrap();

    let default = analyze(&base.to_string_lossy());
    let with_fragments = analyze_opts(
        &base.to_string_lossy(),
        &Options {
            include_fragments: true,
            ..Options::default()
        },
    );

    std::fs::remove_dir_all(&base).ok();

    // Default: solo la regola sugli elementi (img-alt), niente title/charset/…
    assert_eq!(count_rule(&default.findings, "img-alt"), 1);
    for doc_rule in [
        "title-present",
        "meta-charset",
        "meta-viewport",
        "single-h1",
    ] {
        assert_eq!(
            count_rule(&default.findings, doc_rule),
            0,
            "la regola di documento {doc_rule} non deve scattare su un frammento"
        );
    }

    // Con --include-fragments le regole di documento tornano attive.
    assert_eq!(count_rule(&with_fragments.findings, "img-alt"), 1);
    assert!(count_rule(&with_fragments.findings, "title-present") >= 1);
    assert!(count_rule(&with_fragments.findings, "meta-charset") >= 1);
}

/// `--only-category seo` deve eseguire solo le regole SEO; `--preset a11y` solo
/// quelle di accessibilità.
#[test]
fn filtra_per_categoria_e_preset() {
    let seo_only = analyze_opts(
        FIXTURES,
        &Options {
            only_categories: vec!["seo".to_string()],
            ..Options::default()
        },
    );
    // Nessun finding di accessibilità/altra categoria fra i risultati SEO.
    for a11y in ["img-alt", "html-lang", "a-no-text"] {
        assert_eq!(count_rule(&seo_only.findings, a11y), 0, "{a11y} non è SEO");
    }
    assert!(
        seo_only.findings.iter().any(|f| f.rule == "meta-charset"),
        "atteso almeno un finding SEO"
    );

    let a11y_only = analyze_opts(
        FIXTURES,
        &Options {
            preset: Some("a11y".to_string()),
            ..Options::default()
        },
    );
    for seo in ["meta-charset", "meta-viewport", "meta-description"] {
        assert_eq!(count_rule(&a11y_only.findings, seo), 0, "{seo} non è a11y");
    }
    assert!(a11y_only.findings.iter().any(|f| f.rule == "img-alt"));
}

/// Le regole introdotte in Fase 3 devono scattare sulle rispettive violazioni.
/// Usiamo pagine complete in una cartella temporanea e il preset `all` per
/// includere anche le regole opt-in (img-lazy-loading).
#[test]
fn le_nuove_regole_scattano() {
    let base = std::env::temp_dir().join(format!("lightship-newrules-{}", std::process::id()));
    std::fs::create_dir_all(&base).unwrap();
    let page = |body: &str| {
        format!(
            "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\
             <meta name=\"viewport\" content=\"width=device-width\"><title>t</title></head>\
             <body>{body}</body></html>"
        )
    };
    let write = |name: &str, body: &str| std::fs::write(base.join(name), page(body)).unwrap();

    write("tabindex.html", r#"<button tabindex="3">x</button>"#);
    write("heading.html", "<h1>a</h1><h4>b</h4>");
    write(
        "labelfor.html",
        r#"<label for="missing">L</label><input id="other">"#,
    );
    write(
        "ariaref.html",
        r#"<button aria-labelledby="nope">x</button>"#,
    );
    write(
        "tableheaders.html",
        "<table><tr><td>only td</td></tr></table>",
    );
    write(
        "lazy.html",
        r#"<img src="a.png" alt="a" width="1" height="1">"#,
    );
    // lang non valido: sovrascriviamo il documento con lang="english".
    std::fs::write(
        base.join("langinvalid.html"),
        r#"<!DOCTYPE html><html lang="english"><head><meta charset="utf-8"><title>t</title></head><body><p>x</p></body></html>"#,
    )
    .unwrap();
    // duplicate meta: due <title>.
    std::fs::write(
        base.join("dupmeta.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><title>a</title><title>b</title></head><body><p>x</p></body></html>"#,
    )
    .unwrap();

    let all = analyze_opts(
        &base.to_string_lossy(),
        &Options {
            preset: Some("all".to_string()),
            ..Options::default()
        },
    );
    // Preset di default (recommended): img-lazy-loading è opt-in, non compare.
    let recommended = analyze(&base.to_string_lossy());
    std::fs::remove_dir_all(&base).ok();

    for rule in [
        "positive-tabindex",
        "heading-order",
        "label-for-target",
        "aria-ref-target",
        "table-headers",
        "lang-valid",
        "duplicate-meta",
        "img-lazy-loading",
    ] {
        assert!(
            count_rule(&all.findings, rule) >= 1,
            "la regola {rule} avrebbe dovuto scattare"
        );
    }
    assert_eq!(count_rule(&recommended.findings, "img-lazy-loading"), 0);
    assert!(count_rule(&recommended.findings, "positive-tabindex") >= 1);
}

/// Dopo aver scritto un baseline, i finding esistenti vengono soppressi; un
/// problema **nuovo** invece emerge.
#[test]
fn il_baseline_sopprime_i_finding_noti() {
    let base = std::env::temp_dir().join(format!("lightship-baseline-{}", std::process::id()));
    std::fs::create_dir_all(&base).unwrap();
    // Pagina altrimenti "pulita" (h1, description valida), così scatta solo img-alt.
    let doc = |body: &str| {
        format!(
            "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\
             <meta name=\"viewport\" content=\"width=device-width\">\
             <meta name=\"description\" content=\"A sufficiently long description of the test page, well within the SEO range.\">\
             <title>t</title></head>\
             <body><h1>Title</h1>{body}</body></html>"
        )
    };
    // Stato iniziale: un'immagine senza alt (width/height/loading impostati così
    // scatta solo img-alt e il conteggio resta pulito).
    let img = r#"<img src="a.png" width="1" height="1" loading="lazy">"#;
    std::fs::write(base.join("index.html"), doc(img)).unwrap();

    // Congela il baseline e scrivilo dove l'analisi lo cerca.
    let baseline = build_baseline(&base.to_string_lossy(), &Options::default());
    std::fs::write(base.join(BASELINE_FILE), baseline.to_json()).unwrap();

    // Ora l'analisi non conta più il finding preesistente.
    let after = analyze(&base.to_string_lossy());
    assert_eq!(count_rule(&after.findings, "img-alt"), 0);
    assert_eq!(after.baselined, 1);

    // Un nuovo problema (seconda img senza alt) deve emergere: il baseline
    // copriva **una** occorrenza, quindi la seconda resta come finding nuovo.
    std::fs::write(base.join("index.html"), doc(&format!("{img}{img}"))).unwrap();
    let regressed = analyze(&base.to_string_lossy());
    std::fs::remove_dir_all(&base).ok();
    assert_eq!(
        count_rule(&regressed.findings, "img-alt"),
        1,
        "la seconda img (nuova) non deve essere soppressa dal baseline"
    );
    assert_eq!(regressed.baselined, 1);
}

/// L'autofix deve risolvere i problemi con una correzione deterministica
/// (rel/noopener, loading, meta charset/viewport, lang) e lasciar intatti quelli
/// che richiedono contenuto umano.
#[test]
fn autofix_risolve_i_problemi_sicuri() {
    let base = std::env::temp_dir().join(format!("lightship-autofix-{}", std::process::id()));
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(
        base.join("page.html"),
        r#"<!DOCTYPE html><html><head><title>T</title></head><body><h1>Hi</h1><a href="/x" target="_blank">go</a><img src="a.png" alt="a" width="1" height="1"></body></html>"#,
    )
    .unwrap();

    let opts = Options {
        preset: Some("all".to_string()),
        ..Options::default()
    };
    let fixes = collect_fixes(&base.to_string_lossy(), &opts);
    assert!(!fixes.is_empty(), "attesi dei fix proponibili");

    let refs: Vec<&Fix> = fixes.iter().collect();
    let files = apply_fixes(&refs).expect("apply");
    assert_eq!(files, 1);

    let after = analyze_opts(&base.to_string_lossy(), &opts);
    std::fs::remove_dir_all(&base).ok();
    for rule in [
        "link-target-blank",
        "img-lazy-loading",
        "meta-charset",
        "meta-viewport",
        "html-lang",
    ] {
        assert_eq!(
            count_rule(&after.findings, rule),
            0,
            "il fix di {rule} avrebbe dovuto risolvere il problema"
        );
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
