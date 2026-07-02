use crate::finding::{Finding, Severity};
use crate::report::Glyphs;
use owo_colors::OwoColorize;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Larghezza fissa a cui vengono espansi i tab nello snippet, così
/// l'allineamento del caret non dipende dal terminale.
const TAB_WIDTH: usize = 4;

/// Renderizza lo snippet di codice (riga incriminata + sottolineatura) indentato
/// di `indent` spazi. `max_line` è la larghezza massima della finestra mostrata:
/// l'HTML buildato è spesso minificato su una sola riga lunghissima, quindi
/// mostriamo una finestra **centrata sul token** (con ellissi sui lati tagliati);
/// se il tag stesso è più largo della finestra, lo mostriamo dal suo inizio.
/// Tutte le misure sono in larghezza di display (unicode-width), così il caret
/// resta allineato anche con CJK/emoji. Stringa vuota se il finding non ha span.
pub fn render(finding: &Finding, indent: usize, g: &Glyphs, max_line: usize) -> String {
    let Some((offset, len)) = finding.span else {
        return String::new();
    };
    let src = &*finding.source;
    if offset > src.len() {
        return String::new();
    }

    let line_start = src[..offset].rfind('\n').map_or(0, |i| i + 1);
    let line_end = src[offset..].find('\n').map_or(src.len(), |i| offset + i);
    // Numero di riga già calcolato in `attach_with`; fallback al conteggio diretto
    // per i finding costruiti senza orchestratore (es. nei test).
    let line_no = finding
        .line
        .unwrap_or_else(|| src[..offset].bytes().filter(|&b| b == b'\n').count() + 1);

    // Riga con i tab espansi: il token è individuato da indici (in caratteri)
    // nella forma espansa, così finestra e caret restano coerenti.
    let mut chars: Vec<char> = Vec::new();
    expand(&src[line_start..offset.min(line_end)], &mut chars);
    let token_start = chars.len();
    let token_src_end = (offset + len).min(line_end).max(offset);
    expand(&src[offset..token_src_end], &mut chars);
    let token_end = chars.len();
    expand(&src[token_src_end..line_end], &mut chars);

    // 2 colonne riservate alle eventuali ellissi ai bordi.
    let budget = max_line.max(40).saturating_sub(2);
    let (start, end) = window(&chars, token_start, token_end, budget);
    let lead = if start > 0 { g.ellipsis } else { "" };
    let tail = if end < chars.len() { g.ellipsis } else { "" };
    let shown: String = chars[start..end].iter().collect();
    let shown = if tail.is_empty() {
        shown.trim_end()
    } else {
        &shown
    };

    // Posizione e larghezza del caret in **colonne di display**.
    let caret_pad = display_width(&chars[start..token_start]) + lead.width();
    let token_cols = display_width(&chars[token_start..token_end.min(end)]).max(1);

    let pad = " ".repeat(indent);
    let gutter = line_no.to_string();
    let gutter_w = gutter.chars().count();

    // Sottolineiamo l'intero tag di apertura (più leggibile del singolo caret).
    let underline = g.caret.repeat(token_cols);
    let carets = match finding.severity {
        Severity::Error => underline.red().to_string(),
        Severity::Warn => underline.yellow().to_string(),
    };

    format!(
        "{pad}{gutter} {bar} {lead}{line}{tail}\n{pad}{blank} {bar} {caret_space}{carets} {qui}\n",
        gutter = gutter.dimmed(),
        bar = g.bar.dimmed(),
        blank = " ".repeat(gutter_w),
        line = shown,
        caret_space = " ".repeat(caret_pad),
        qui = "here".dimmed(),
    )
}

/// Accoda `s` a `out` espandendo ogni tab a [`TAB_WIDTH`] spazi.
fn expand(s: &str, out: &mut Vec<char>) {
    for c in s.chars() {
        if c == '\t' {
            out.extend(std::iter::repeat_n(' ', TAB_WIDTH));
        } else {
            out.push(c);
        }
    }
}

/// Larghezza di display di una slice di caratteri (i control char contano 0).
fn display_width(chars: &[char]) -> usize {
    chars.iter().map(|&c| c.width().unwrap_or(0)).sum()
}

/// Sceglie la finestra `[start, end)` da mostrare: se la riga sta in `max_w`
/// colonne la mostra intera, altrimenti parte dal token e si espande alternando
/// sinistra e destra finché c'è budget, così il token resta **centrato** nel
/// contesto disponibile. Se il token da solo eccede il budget, la finestra
/// inizia al suo inizio (si vede il tag di apertura fino a dove ci sta).
fn window(chars: &[char], token_start: usize, token_end: usize, max_w: usize) -> (usize, usize) {
    if display_width(chars) <= max_w {
        return (0, chars.len());
    }

    let mut start = token_start.min(chars.len());
    let mut end = start;
    let mut used = 0usize;

    // Prima il token, finché sta nel budget.
    while end < token_end.min(chars.len()) {
        let w = chars[end].width().unwrap_or(0);
        if used + w > max_w {
            break;
        }
        used += w;
        end += 1;
    }

    // Poi il contesto, un carattere per lato alla volta: il token resta centrato
    // e ai bordi della riga il budget residuo va tutto al lato opposto.
    loop {
        let mut grew = false;
        if start > 0 {
            let w = chars[start - 1].width().unwrap_or(0);
            if used + w <= max_w {
                start -= 1;
                used += w;
                grew = true;
            }
        }
        if end < chars.len() {
            let w = chars[end].width().unwrap_or(0);
            if used + w <= max_w {
                used += w;
                end += 1;
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars_of(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    /// Rimuove le sequenze ANSI `ESC[...m` per poter asserire sul testo puro.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut it = s.chars();
        while let Some(c) = it.next() {
            if c == '\x1b' {
                for d in it.by_ref() {
                    if d == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn riga_corta_mostrata_intera() {
        let chars = chars_of("<p><img></p>");
        assert_eq!(window(&chars, 3, 8, 40), (0, chars.len()));
    }

    #[test]
    fn riga_minificata_centra_il_token() {
        // Token di 5 char in mezzo a una riga di 200: la finestra da 41 (budget
        // dispari: 5 + 18*2) deve avere il token circa al centro.
        let mut line = "a".repeat(100);
        line.push_str("<img>");
        line.push_str(&"b".repeat(100));
        let chars = chars_of(&line);
        let (start, end) = window(&chars, 100, 105, 41);
        assert_eq!(end - start, 41);
        let left = 100 - start;
        let right = end - 105;
        assert!(left.abs_diff(right) <= 1, "left {left} right {right}");
    }

    #[test]
    fn token_a_inizio_riga_da_il_budget_alla_destra() {
        let mut line = "<img>".to_string();
        line.push_str(&"b".repeat(200));
        let chars = chars_of(&line);
        let (start, end) = window(&chars, 0, 5, 40);
        assert_eq!(start, 0);
        assert_eq!(end, 40);
    }

    #[test]
    fn token_piu_largo_della_finestra_parte_dal_suo_inizio() {
        let mut line = "a".repeat(50);
        line.push_str(&"<img src=\"data:x\">".repeat(10)); // token enorme
        let chars = chars_of(&line);
        let (start, _) = window(&chars, 50, 50 + 180, 40);
        assert_eq!(start, 50);
    }

    #[test]
    fn larghezza_display_con_cjk() {
        // I CJK contano 2 colonne: il budget in colonne deve dimezzare i char.
        let line = "中".repeat(100);
        let chars = chars_of(&line);
        let (start, end) = window(&chars, 50, 51, 40);
        assert!(display_width(&chars[start..end]) <= 40);
        assert!(end - start >= 19); // ~20 char da 2 colonne
    }

    #[test]
    fn caret_allineato_dopo_prefisso_cjk() {
        use crate::finding::Severity;
        use std::sync::Arc;
        // Prefisso largo (CJK): il padding del caret deve usare la larghezza di
        // display (8 colonne per 4 char), non il conteggio caratteri.
        let src = "中中中中<img>";
        let off = src.find('<').unwrap();
        let mut f = Finding::new("x", Severity::Warn, "m", Some((off, 5)));
        f.attach(std::path::PathBuf::from("a.html"), Arc::from(src));
        let out = strip_ansi(&render(&f, 0, &Glyphs::new(false), 80));
        let caret_line = out.lines().nth(1).unwrap();
        // Dopo "gutter-blank │ " ci aspettiamo 8 spazi prima del caret.
        let after_bar = caret_line.split('│').nth(1).unwrap();
        let spaces = after_bar.chars().skip(1).take_while(|&c| c == ' ').count();
        assert_eq!(spaces, 8);
    }

    #[test]
    fn tab_espansi_a_larghezza_fissa() {
        use crate::finding::Severity;
        use std::sync::Arc;
        let src = "\t<img>";
        let off = src.find('<').unwrap();
        let mut f = Finding::new("x", Severity::Warn, "m", Some((off, 5)));
        f.attach(std::path::PathBuf::from("a.html"), Arc::from(src));
        let out = render(&f, 0, &Glyphs::new(false), 80);
        let first = out.lines().next().unwrap();
        assert!(first.contains(&format!("{}<img>", " ".repeat(TAB_WIDTH))));
    }
}
