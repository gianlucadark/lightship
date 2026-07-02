//! Baseline / soppressione dei finding già noti.
//!
//! Adottare un linter su un progetto esistente genera spesso decine di finding
//! preesistenti: bloccare subito la CID sarebbe impraticabile. Un **baseline**
//! congela lo stato attuale (`lightship baseline`) così che l'analisi successiva
//! fallisca solo sui problemi **nuovi**, lasciando emergere i regressi senza
//! nascondere per sempre quelli vecchi.

use crate::finding::{Finding, fingerprint_of};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Nome del file di baseline cercato nella cartella analizzata / cwd.
pub const BASELINE_FILE: &str = "lightship-baseline.json";

/// Una voce del baseline: abbastanza leggibile da poter essere rivista in code
/// review, e sufficiente a ricostruire il fingerprint.
///
/// `snippet` (formato v2) è il codice reale a cui punta il finding: è la parte
/// che entra nel fingerprint quando presente, perché resta stabile anche se il
/// wording del messaggio cambia. È assente nei baseline v1 e nei finding "di
/// documento" senza span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub rule: String,
    pub file: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

/// Contenuto del file di baseline.
#[derive(Debug, Serialize, Deserialize)]
pub struct Baseline {
    pub version: u32,
    pub entries: Vec<Entry>,
}

impl Baseline {
    /// Versione corrente del formato (v2: fingerprint basato sullo snippet).
    pub const VERSION: u32 = 2;

    /// Costruisce un baseline dai finding correnti (path normalizzati a `/`,
    /// ordine deterministico).
    pub fn from_findings(findings: &[Finding]) -> Baseline {
        let mut entries: Vec<Entry> = findings
            .iter()
            .map(|f| Entry {
                rule: f.rule.to_string(),
                file: f.file.display().to_string().replace('\\', "/"),
                message: f.message.clone(),
                snippet: f.snippet().map(str::to_string),
            })
            .collect();
        entries.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then_with(|| a.rule.cmp(&b.rule))
                .then_with(|| a.message.cmp(&b.message))
                .then_with(|| a.snippet.cmp(&b.snippet))
        });
        Baseline {
            version: Self::VERSION,
            entries,
        }
    }

    /// `true` se il file è nel vecchio formato v1: i suoi fingerprint derivano
    /// dal messaggio, quindi i finding vanno matchati con
    /// [`Finding::fingerprint_v1`](crate::finding::Finding::fingerprint_v1).
    pub fn is_legacy(&self) -> bool {
        self.version < Self::VERSION
    }

    /// Serializza il baseline in JSON indentato.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Carica un baseline da `path`, se esiste ed è valido.
    pub fn load(path: &Path) -> Option<Baseline> {
        let raw = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    /// Il multiset dei fingerprint coperti dal baseline: `fingerprint → quante
    /// occorrenze`. Contare le occorrenze fa emergere un problema **nuovo** anche
    /// quando è identico a uno già noto nello stesso file (es. una seconda `<img>`
    /// senza alt): il baseline ne copre solo quante ne aveva congelate.
    ///
    /// Nel formato v2 il fingerprint deriva dallo snippet quando presente; nelle
    /// voci v1 `snippet` è assente, quindi il fallback sul messaggio riproduce
    /// esattamente i fingerprint v1.
    pub fn fingerprint_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for e in &self.entries {
            let text = e.snippet.as_deref().unwrap_or(&e.message);
            *counts
                .entry(fingerprint_of(&e.rule, &e.file, text))
                .or_insert(0) += 1;
        }
        counts
    }
}

/// Percorso del baseline da usare: `explicit` (`--baseline`), poi
/// `<dir>/lightship-baseline.json`, poi `./lightship-baseline.json`. `None` se
/// nessuno esiste.
pub fn resolve(dir: &str, explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }
    [
        Path::new(dir).join(BASELINE_FILE),
        PathBuf::from(BASELINE_FILE),
    ]
    .into_iter()
    .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::Severity;
    use std::path::PathBuf;

    fn finding(rule: &'static str, file: &str, msg: &str) -> Finding {
        let mut f = Finding::new(rule, Severity::Warn, msg.to_string(), None);
        f.file = PathBuf::from(file);
        f
    }

    /// Finding con span reale, così `snippet()` ritorna il codice puntato.
    fn finding_with_span(rule: &'static str, file: &str, msg: &str, src: &str) -> Finding {
        let off = src.find('<').unwrap();
        let mut f = Finding::new(rule, Severity::Warn, msg.to_string(), Some((off, src.len() - off)));
        f.attach(PathBuf::from(file), std::sync::Arc::from(src));
        f
    }

    #[test]
    fn round_trip_e_fingerprint_stabile() {
        let findings = vec![finding("img-alt", "dist/a.html", "m1")];
        let base = Baseline::from_findings(&findings);
        let json = base.to_json();
        let loaded = serde_json::from_str::<Baseline>(&json).unwrap();
        // Il fingerprint ricostruito dal baseline coincide con quello del finding.
        assert!(
            loaded
                .fingerprint_counts()
                .contains_key(&findings[0].fingerprint())
        );
    }

    #[test]
    fn path_windows_e_unix_coincidono() {
        let win = finding("img-alt", "dist\\a.html", "m");
        let base = Baseline::from_findings(std::slice::from_ref(&win));
        assert!(base.fingerprint_counts().contains_key(&win.fingerprint()));
    }

    #[test]
    fn conta_le_occorrenze() {
        // Due finding identici ⇒ conteggio 2.
        let findings = vec![
            finding("img-alt", "dist/a.html", "m"),
            finding("img-alt", "dist/a.html", "m"),
        ];
        let counts = Baseline::from_findings(&findings).fingerprint_counts();
        assert_eq!(counts.values().sum::<usize>(), 2);
    }

    #[test]
    fn fingerprint_v2_regge_il_cambio_di_messaggio() {
        // Stesso snippet, wording del messaggio diverso ⇒ stesso fingerprint v2.
        let a = finding_with_span("img-alt", "dist/a.html", "old wording", "<img src=\"a.png\">");
        let b = finding_with_span("img-alt", "dist/a.html", "new wording", "<img src=\"a.png\">");
        assert_eq!(a.fingerprint(), b.fingerprint());
        assert_ne!(a.fingerprint_v1(), b.fingerprint_v1());
        // Il baseline costruito da `a` copre anche `b`.
        let base = Baseline::from_findings(std::slice::from_ref(&a));
        assert!(!base.is_legacy());
        assert!(base.fingerprint_counts().contains_key(&b.fingerprint()));
    }

    #[test]
    fn baseline_v1_legacy_matcha_il_fingerprint_v1() {
        // Un file v1 (senza campo `snippet`) deve ancora sopprimere i finding
        // tramite il fingerprint legacy basato sul messaggio.
        let json = r#"{
            "version": 1,
            "entries": [{ "rule": "img-alt", "file": "dist/a.html", "message": "m" }]
        }"#;
        let base = serde_json::from_str::<Baseline>(json).unwrap();
        assert!(base.is_legacy());
        let f = finding_with_span("img-alt", "dist/a.html", "m", "<img>");
        assert!(base.fingerprint_counts().contains_key(&f.fingerprint_v1()));
    }

    #[test]
    fn round_trip_v2_con_snippet() {
        let f = finding_with_span("img-alt", "dist/a.html", "m", "<img src=\"a.png\">");
        let base = Baseline::from_findings(std::slice::from_ref(&f));
        let loaded = serde_json::from_str::<Baseline>(&base.to_json()).unwrap();
        assert_eq!(loaded.version, Baseline::VERSION);
        assert!(loaded.fingerprint_counts().contains_key(&f.fingerprint()));
    }
}
