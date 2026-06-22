//! Rilevamento automatico della cartella di build da analizzare.
//!
//! Un utente medio non sa (o non vuole ricordare) che il target è `dist/` /
//! `out/` / `_site/`… Quando `lightship` gira senza argomenti, proviamo a
//! dedurre framework e cartella di output dal progetto nella cwd, così
//! "just works" senza configurazione.

use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Esito del rilevamento: la cartella di output trovata e, se noto, il
/// framework che l'ha prodotta (solo per messaggistica).
pub struct Detected {
    /// Nome del framework rilevato, se identificato (es. `"Astro"`).
    pub framework: Option<&'static str>,
    /// Cartella di output esistente che contiene almeno un `.html`.
    pub dir: PathBuf,
}

/// Cartelle di output da provare in fallback, in ordine di preferenza.
const FALLBACK_DIRS: &[&str] = &["dist", "build", "out", "_site", "public", ".output/public"];

/// Cerca una cartella di build sotto `root`.
///
/// Strategia: prima si deduce il framework (dalle dipendenze di `package.json`
/// o dai file di config) e si prova la sua cartella tipica; poi si provano i
/// candidati standard. Vince la prima cartella che **esiste** e contiene almeno
/// un `.html`. `None` se non si trova nulla (es. progetto non ancora buildato).
pub fn detect_build_dir(root: &Path) -> Option<Detected> {
    // Candidati ordinati: prima quello del framework (con nome), poi i default.
    let mut candidates: Vec<(Option<&'static str>, &str)> = Vec::new();
    if let Some((name, dir)) = detect_framework(root) {
        candidates.push((Some(name), dir));
    }
    for &dir in FALLBACK_DIRS {
        if !candidates.iter().any(|(_, c)| *c == dir) {
            candidates.push((None, dir));
        }
    }

    for (framework, rel) in candidates {
        let path = root.join(rel);
        if path.is_dir() && has_html(&path) {
            return Some(Detected { framework, dir: path });
        }
    }
    None
}

/// Deduce `(framework, cartella-output-tipica)` da `package.json` o dai file di
/// config presenti in `root`. `None` se non si riconosce nulla.
pub fn detect_framework(root: &Path) -> Option<(&'static str, &'static str)> {
    if let Some(fw) = from_package_json(root) {
        return Some(fw);
    }
    from_config_files(root)
}

/// Dipendenze di `package.json` → framework. Le più specifiche prima (Astro e
/// SvelteKit dipendono da Vite, quindi vanno controllate per prime).
fn from_package_json(root: &Path) -> Option<(&'static str, &'static str)> {
    let raw = std::fs::read_to_string(root.join("package.json")).ok()?;
    // Alcuni tool scrivono il file con un BOM UTF-8 in testa, che serde_json non
    // salta: lo togliamo per non fallire il parse.
    let json: serde_json::Value = serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok()?;

    let has = |name: &str| {
        ["dependencies", "devDependencies"]
            .iter()
            .any(|section| json.get(section).and_then(|d| d.get(name)).is_some())
    };

    // (dipendenza, framework, output dir) — ordine = priorità.
    const DEPS: &[(&str, &str, &str)] = &[
        ("astro", "Astro", "dist"),
        ("@sveltejs/kit", "SvelteKit", "build"),
        ("next", "Next.js", "out"),
        ("nuxt", "Nuxt", ".output/public"),
        ("gatsby", "Gatsby", "public"),
        ("@11ty/eleventy", "Eleventy", "_site"),
        ("vite", "Vite", "dist"),
    ];
    DEPS.iter()
        .find(|(dep, _, _)| has(dep))
        .map(|&(_, name, dir)| (name, dir))
}

/// File di config alla radice → framework, per progetti senza `package.json`
/// (Hugo, Jekyll) o quando le dipendenze non bastano.
fn from_config_files(root: &Path) -> Option<(&'static str, &'static str)> {
    let exists = |name: &str| root.join(name).exists();
    let any = |names: &[&str]| names.iter().any(|n| exists(n));

    if any(&["astro.config.mjs", "astro.config.js", "astro.config.ts"]) {
        Some(("Astro", "dist"))
    } else if any(&["next.config.js", "next.config.mjs", "next.config.ts"]) {
        Some(("Next.js", "out"))
    } else if any(&["svelte.config.js", "svelte.config.mjs"]) {
        Some(("SvelteKit", "build"))
    } else if any(&["nuxt.config.js", "nuxt.config.ts"]) {
        Some(("Nuxt", ".output/public"))
    } else if any(&["vite.config.js", "vite.config.mjs", "vite.config.ts"]) {
        Some(("Vite", "dist"))
    } else if any(&["hugo.toml", "hugo.yaml", "hugo.yml", "config.toml"]) {
        Some(("Hugo", "public"))
    } else if any(&["_config.yml", "_config.toml"]) {
        Some(("Jekyll", "_site"))
    } else if any(&[".eleventy.js", "eleventy.config.js", "eleventy.config.mjs"]) {
        Some(("Eleventy", "_site"))
    } else {
        None
    }
}

/// `true` se `dir` contiene almeno un file `.html` (ricorsivo). Corto-circuita
/// al primo match per non camminare cartelle enormi inutilmente.
fn has_html(dir: &Path) -> bool {
    WalkBuilder::new(dir)
        .standard_filters(false)
        .hidden(true)
        .build()
        .filter_map(Result::ok)
        .any(|e| {
            e.file_type().is_some_and(|t| t.is_file())
                && e.path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("html"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lightship-detect-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn astro_da_package_json() {
        let root = tmp("astro");
        fs::write(
            root.join("package.json"),
            r#"{"devDependencies":{"astro":"^4"}}"#,
        )
        .unwrap();
        assert_eq!(detect_framework(&root), Some(("Astro", "dist")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn jekyll_da_config_file() {
        let root = tmp("jekyll");
        fs::write(root.join("_config.yml"), "title: x\n").unwrap();
        assert_eq!(detect_framework(&root), Some(("Jekyll", "_site")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn trova_la_cartella_con_html() {
        let root = tmp("withdist");
        let dist = root.join("dist");
        fs::create_dir_all(&dist).unwrap();
        fs::write(dist.join("index.html"), "<html></html>").unwrap();
        let found = detect_build_dir(&root).expect("dovrebbe trovare dist");
        assert!(found.dir.ends_with("dist"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn niente_html_niente_detect() {
        let root = tmp("empty");
        fs::create_dir_all(root.join("dist")).unwrap();
        assert!(detect_build_dir(&root).is_none());
        let _ = fs::remove_dir_all(&root);
    }
}
