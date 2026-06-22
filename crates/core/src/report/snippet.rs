use crate::finding::{Finding, Severity};
use owo_colors::OwoColorize;

/// Larghezza massima della riga mostrata: l'HTML buildato è spesso minificato su
/// una sola riga lunghissima, quindi mostriamo una finestra attorno al token.
const MAX_LINE: usize = 140;

/// Renderizza lo snippet di codice (riga incriminata + caret) indentato di
/// `indent` spazi. Stringa vuota se il finding non ha span.
pub fn render(finding: &Finding, indent: usize) -> String {
    let Some((offset, len)) = finding.span else {
        return String::new();
    };
    let src = &*finding.source;
    if offset > src.len() {
        return String::new();
    }

    let line_start = src[..offset].rfind('\n').map_or(0, |i| i + 1);
    let line_end = src[offset..].find('\n').map_or(src.len(), |i| offset + i);
    let line_text = &src[line_start..line_end];
    let line_no = src[..offset].bytes().filter(|&b| b == b'\n').count() + 1;

    // Colonna (in caratteri) del token nella riga. Un singolo caret che punta
    // all'inizio del tag è più pulito che sottolinearlo per intero.
    let _ = len;
    let col = src[line_start..offset].chars().count();

    let (shown, col, trimmed_left) = window(line_text, col);
    let pad = " ".repeat(indent);
    let gutter = line_no.to_string();
    let gutter_w = gutter.chars().count();

    let carets = match finding.severity {
        Severity::Error => "▲".red().to_string(),
        Severity::Warn => "▲".yellow().to_string(),
    };
    let lead = if trimmed_left { "…" } else { "" };

    format!(
        "{pad}{gutter} {bar} {lead}{line}\n{pad}{blank} {bar} {caret_pad}{carets} {qui}\n",
        gutter = gutter.dimmed(),
        bar = "│".dimmed(),
        blank = " ".repeat(gutter_w),
        line = shown,
        caret_pad = " ".repeat(col + lead.chars().count()),
        qui = "qui".dimmed(),
    )
}

/// Restituisce una finestra di al più `MAX_LINE` caratteri attorno alla colonna
/// `col`, la nuova colonna relativa alla finestra e se è stato tagliato a
/// sinistra (così da prefissare un `…`). I tab diventano spazi per allineare.
fn window(line: &str, col: usize) -> (String, usize, bool) {
    let chars: Vec<char> = line
        .chars()
        .map(|c| if c == '\t' { ' ' } else { c })
        .collect();
    if chars.len() <= MAX_LINE {
        let s: String = chars.into_iter().collect();
        return (s.trim_end().to_string(), col, false);
    }
    // Mostra un po' di contesto prima del token.
    let start = col.saturating_sub(20);
    let end = (start + MAX_LINE).min(chars.len());
    let slice: String = chars[start..end].iter().collect();
    (slice.trim_end().to_string(), col - start, start > 0)
}
