//! Autofix: modifiche testuali **sicure e deterministiche** proposte dalle
//! regole, applicate solo su scelta esplicita dell'utente.
//!
//! Un [`Fix`] descrive *cosa* verrà cambiato (messaggio + posizione) e *come*
//! (un [`Edit`] sul sorgente). Le regole producono fix solo quando esiste una
//! correzione non ambigua (es. aggiungere `rel="noopener"`); dove servirebbe
//! giudizio umano (il testo `alt`, le dimensioni immagine) **non** si propone
//! nulla e il problema resta un semplice finding.

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;

/// Una sostituzione testuale sul sorgente: `[start, end)` in byte diventa
/// `replacement`. Un inserimento puro ha `start == end`.
#[derive(Debug, Clone)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

impl Edit {
    /// Inserisce `text` all'offset `at` senza rimuovere nulla.
    pub fn insert(at: usize, text: impl Into<String>) -> Edit {
        Edit {
            start: at,
            end: at,
            replacement: text.into(),
        }
    }

    /// Sostituisce l'intervallo `[start, end)` con `text`.
    pub fn replace(start: usize, end: usize, text: impl Into<String>) -> Edit {
        Edit {
            start,
            end,
            replacement: text.into(),
        }
    }
}

/// Un fix proposto da una regola: il *cosa* (messaggio + posizione per il numero
/// di riga) e il *come* (l'[`Edit`]). `file`/`source`/`line`/`column` sono
/// riempiti dall'orchestratore, come per i finding.
#[derive(Debug, Clone, Serialize)]
pub struct Fix {
    pub rule: &'static str,
    /// Cosa fa il fix, in una riga (mostrato nella lista numerata).
    pub message: String,
    /// Anteprima leggibile del testo inserito/sostituito (es. `rel="noopener"`).
    pub preview: String,
    #[serde(skip)]
    pub edit: Edit,
    pub file: PathBuf,
    pub line: Option<usize>,
    pub column: Option<usize>,
    #[serde(skip)]
    pub source: Arc<str>,
}

impl Fix {
    /// Crea un fix senza file/sorgente (li aggancia poi l'orchestratore). `anchor`
    /// è l'offset usato per calcolare riga/colonna nella lista.
    pub fn new(
        rule: &'static str,
        message: impl Into<String>,
        preview: impl Into<String>,
        edit: Edit,
    ) -> Fix {
        Fix {
            rule,
            message: message.into(),
            preview: preview.into(),
            edit,
            file: PathBuf::new(),
            line: None,
            column: None,
            source: Arc::from(""),
        }
    }

    /// Aggancia file e sorgente e calcola riga/colonna dall'inizio dell'edit.
    pub fn attach(&mut self, file: PathBuf, source: Arc<str>) {
        let at = self.edit.start.min(source.len());
        let prefix = &source[..at];
        self.line = Some(prefix.bytes().filter(|&b| b == b'\n').count() + 1);
        self.column = Some(at - prefix.rfind('\n').map_or(0, |i| i + 1) + 1);
        self.file = file;
        self.source = source;
    }
}

/// Applica un insieme di edit a `src` e ritorna il nuovo testo. Gli edit vengono
/// applicati **dall'offset più alto al più basso** così che ognuno non invalidi
/// gli offset di quelli successivi. Edit sovrapposti vengono saltati (il primo
/// in ordine di applicazione vince), per non produrre output corrotto.
pub fn apply_edits(src: &str, edits: &[Edit]) -> String {
    let mut edits: Vec<&Edit> = edits.iter().collect();
    edits.sort_by_key(|e| std::cmp::Reverse(e.start));

    let mut out = src.to_string();
    let mut last_start = out.len() + 1;
    for e in edits {
        // Salta edit fuori range o che si sovrappongono a uno già applicato.
        if e.end > out.len() || e.start > e.end || e.end > last_start {
            continue;
        }
        out.replace_range(e.start..e.end, &e.replacement);
        last_start = e.start;
    }
    out
}

/// Interpreta la selezione dell'utente sulla lista numerata (1-based):
/// `"a"`/`"all"` ⇒ tutti; `""`/`"q"`/`"n"` ⇒ nessuno (annulla); altrimenti una
/// lista come `"1,3,5"` o `"2-4"`. Ritorna gli **indici 0-based** validi, oppure
/// `None` se l'utente ha annullato.
pub fn parse_selection(input: &str, count: usize) -> Option<Vec<usize>> {
    let s = input.trim().to_ascii_lowercase();
    match s.as_str() {
        "a" | "all" => return Some((0..count).collect()),
        "" | "q" | "quit" | "n" | "none" => return None,
        _ => {}
    }
    let mut picked = Vec::new();
    for part in s.split([',', ' ']).filter(|p| !p.is_empty()) {
        if let Some((lo, hi)) = part.split_once('-') {
            if let (Ok(lo), Ok(hi)) = (lo.trim().parse::<usize>(), hi.trim().parse::<usize>()) {
                for n in lo..=hi {
                    if (1..=count).contains(&n) {
                        picked.push(n - 1);
                    }
                }
            }
        } else if let Ok(n) = part.parse::<usize>()
            && (1..=count).contains(&n)
        {
            picked.push(n - 1);
        }
    }
    picked.sort_unstable();
    picked.dedup();
    if picked.is_empty() {
        None
    } else {
        Some(picked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_inserimenti_e_sostituzioni() {
        // Inserisce ` rel="noopener"` prima del `>` di <a ...>.
        let src = r#"<a href="/x" target="_blank">x</a>"#;
        let at = src.find('>').unwrap();
        let out = apply_edits(src, &[Edit::insert(at, r#" rel="noopener""#)]);
        assert_eq!(out, r#"<a href="/x" target="_blank" rel="noopener">x</a>"#);
    }

    #[test]
    fn edit_multipli_non_si_invalidano() {
        let src = "AABBB";
        let edits = [Edit::replace(0, 2, "x"), Edit::replace(2, 5, "y")];
        assert_eq!(apply_edits(src, &edits), "xy");
    }

    #[test]
    fn selezione_all_singoli_e_range() {
        assert_eq!(parse_selection("all", 3), Some(vec![0, 1, 2]));
        assert_eq!(parse_selection("1,3", 3), Some(vec![0, 2]));
        assert_eq!(parse_selection("2-3", 3), Some(vec![1, 2]));
        assert_eq!(parse_selection("q", 3), None);
        assert_eq!(parse_selection("", 3), None);
        // Numeri fuori range ignorati.
        assert_eq!(parse_selection("9", 3), None);
    }
}
