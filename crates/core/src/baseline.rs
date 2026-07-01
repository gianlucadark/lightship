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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub rule: String,
    pub file: String,
    pub message: String,
}

/// Contenuto del file di baseline.
#[derive(Debug, Serialize, Deserialize)]
pub struct Baseline {
    pub version: u32,
    pub entries: Vec<Entry>,
}

impl Baseline {
    /// Versione corrente del formato.
    const VERSION: u32 = 1;

    /// Costruisce un baseline dai finding correnti (path normalizzati a `/`,
    /// ordine deterministico).
    pub fn from_findings(findings: &[Finding]) -> Baseline {
        let mut entries: Vec<Entry> = findings
            .iter()
            .map(|f| Entry {
                rule: f.rule.to_string(),
                file: f.file.display().to_string().replace('\\', "/"),
                message: f.message.clone(),
            })
            .collect();
        entries.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then_with(|| a.rule.cmp(&b.rule))
                .then_with(|| a.message.cmp(&b.message))
        });
        Baseline {
            version: Self::VERSION,
            entries,
        }
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
    pub fn fingerprint_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for e in &self.entries {
            *counts
                .entry(fingerprint_of(&e.rule, &e.file, &e.message))
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
}
