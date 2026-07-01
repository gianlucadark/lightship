use crate::finding::{Finding, Severity};
use crate::report::Glyphs;
use owo_colors::OwoColorize;

/// Renderizza lo snippet di codice (riga incriminata + sottolineatura) indentato
/// di `indent` spazi. `max_line` è la larghezza massima della finestra mostrata:
/// l'HTML buildato è spesso minificato su una sola riga lunghissima, quindi
/// mostriamo una finestra attorno al token. Stringa vuota se il finding non ha span.
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
    let line_text = &src[line_start..line_end];
    // Numero di riga già calcolato in `attach_with`; fallback al conteggio diretto
    // per i finding costruiti senza orchestratore (es. nei test).
    let line_no = finding
        .line
        .unwrap_or_else(|| src[..offset].bytes().filter(|&b| b == b'\n').count() + 1);

    // Colonna (in caratteri) del token nella riga e lunghezza del token in
    // caratteri, limitata alla riga corrente (i tag multi-riga rari li tronchiamo).
    let col = src[line_start..offset].chars().count();
    let token_end = (offset + len).min(line_end);
    let token_chars = src[offset..token_end].chars().count().max(1);

    let (shown, col, trimmed_left) = window(line_text, col, max_line.max(40));
    let pad = " ".repeat(indent);
    let gutter = line_no.to_string();
    let gutter_w = gutter.chars().count();

    // Sottolineiamo l'intero tag di apertura (più leggibile del singolo caret).
    let underline = g.caret.repeat(token_chars);
    let carets = match finding.severity {
        Severity::Error => underline.red().to_string(),
        Severity::Warn => underline.yellow().to_string(),
    };
    let lead = if trimmed_left { g.ellipsis } else { "" };

    format!(
        "{pad}{gutter} {bar} {lead}{line}\n{pad}{blank} {bar} {caret_pad}{carets} {qui}\n",
        gutter = gutter.dimmed(),
        bar = g.bar.dimmed(),
        blank = " ".repeat(gutter_w),
        line = shown,
        caret_pad = " ".repeat(col + lead.chars().count()),
        qui = "here".dimmed(),
    )
}

/// Restituisce una finestra di al più `max_line` caratteri attorno alla colonna
/// `col`, la nuova colonna relativa alla finestra e se è stato tagliato a
/// sinistra (così da prefissare un `…`). I tab diventano spazi per allineare.
fn window(line: &str, col: usize, max_line: usize) -> (String, usize, bool) {
    let chars: Vec<char> = line
        .chars()
        .map(|c| if c == '\t' { ' ' } else { c })
        .collect();
    if chars.len() <= max_line {
        let s: String = chars.into_iter().collect();
        return (s.trim_end().to_string(), col, false);
    }
    // Mostra un po' di contesto prima del token.
    let start = col.saturating_sub(20);
    let end = (start + max_line).min(chars.len());
    let slice: String = chars[start..end].iter().collect();
    (slice.trim_end().to_string(), col - start, start > 0)
}
