use std::borrow::Cow;
use std::collections::HashSet;
use tl::{HTMLTag, Parser, VDom};

/// Insieme degli `id` (non vuoti) presenti nel documento. Usato dalle regole che
/// devono risolvere riferimenti (`label[for]`, `aria-labelledby`…).
pub fn collect_ids(dom: &VDom<'_>, parser: &Parser<'_>) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Some(els) = dom.query_selector("[id]") {
        for tag in els.filter_map(|h| h.get(parser)?.as_tag()) {
            if let Some(id) = attr(tag, "id") {
                let id = id.trim();
                if !id.is_empty() {
                    ids.insert(id.to_string());
                }
            }
        }
    }
    ids
}

/// Vero se il sorgente rappresenta una **pagina HTML completa** e non un semplice
/// frammento/partial. Consideriamo pagina qualunque file che dichiari un doctype
/// oppure contenga un elemento `<html>` o `<head>`: i componenti, i template
/// email o i pezzi renderizzati da htmx/Turbo non ne hanno, quindi le regole
/// "di documento" (title/charset/viewport/h1…) non devono scattarci sopra.
pub fn is_full_document(dom: &VDom<'_>, src: &str) -> bool {
    dom.query_selector("html")
        .and_then(|mut it| it.next())
        .is_some()
        || dom
            .query_selector("head")
            .and_then(|mut it| it.next())
            .is_some()
        || has_doctype(src)
}

/// Vero se il sorgente inizia (dopo eventuali spazi/BOM) con `<!doctype …`.
fn has_doctype(src: &str) -> bool {
    let s = src.trim_start_matches('\u{feff}').trim_start();
    s.len() >= 2
        && s.as_bytes()[0] == b'<'
        && s.as_bytes()[1] == b'!'
        && s.get(2..9)
            .is_some_and(|k| k.eq_ignore_ascii_case("doctype"))
}

/// Cerca un attributo per nome con confronto **case-insensitive**.
///
/// I nomi degli attributi HTML sono case-insensitive, ma `tl` li confronta
/// alla lettera; framework come React/Next emettono nomi camelCase (es.
/// `charSet`, `viewBox`), quindi un match esatto darebbe falsi positivi.
/// Ritorna `None` se l'attributo è assente, `Some(valore)` se presente
/// (stringa vuota per gli attributi booleani senza valore).
pub fn attr<'a>(tag: &'a HTMLTag<'a>, name: &str) -> Option<Cow<'a, str>> {
    tag.attributes().iter().find_map(|(k, v)| {
        k.eq_ignore_ascii_case(name)
            .then(|| v.unwrap_or(Cow::Borrowed("")))
    })
}

/// Vero se l'attributo (case-insensitive) esiste, qualunque sia il valore.
pub fn has_attr(tag: &HTMLTag<'_>, name: &str) -> bool {
    tag.attributes()
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case(name))
}

/// Vero se l'attributo (case-insensitive) esiste con un valore non vuoto.
pub fn attr_non_empty(tag: &HTMLTag<'_>, name: &str) -> bool {
    attr(tag, name).is_some_and(|v| !v.trim().is_empty())
}

/// Vero se l'elemento è nascosto all'albero di accessibilità: attributo `hidden`
/// oppure `aria-hidden="true"`. Per questi elementi non ha senso segnalare la
/// mancanza di testo alternativo o di un nome accessibile.
pub fn is_a11y_hidden(tag: &HTMLTag<'_>) -> bool {
    has_attr(tag, "hidden")
        || attr(tag, "aria-hidden").is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

/// Vero se l'elemento ha un ruolo "presentazionale" (`role="presentation"` o
/// `role="none"`): è esposto come puramente decorativo, quindi non richiede un
/// testo alternativo.
pub fn is_presentational(tag: &HTMLTag<'_>) -> bool {
    attr(tag, "role").is_some_and(|v| {
        let r = v.trim();
        r.eq_ignore_ascii_case("presentation") || r.eq_ignore_ascii_case("none")
    })
}

/// Lo span `(offset, len)` in byte del **solo tag di apertura** dell'elemento
/// nel sorgente, es. `<img src="hero.png">`.
///
/// `tl::HTMLTag::boundaries` dà l'intervallo dell'intero elemento (per `<html>`
/// sarebbe l'intero documento), quindi lo tronchiamo al primo `>` di chiusura
/// del tag di apertura. Lo span è riferito allo stesso `src` parsato da `tl`,
/// così l'offset cade esattamente sul codice reale.
pub fn opening_tag_span(tag: &HTMLTag<'_>, parser: &Parser<'_>, src: &str) -> (usize, usize) {
    let (start, _) = tag.boundaries(parser);
    (start, open_tag_len(src, start))
}

/// Lunghezza (in byte) del tag di apertura a partire da `start`: dal `<` fino al
/// primo `>` che non sia dentro un valore di attributo quotato.
fn open_tag_len(src: &str, start: usize) -> usize {
    let bytes = src.as_bytes();
    let mut quote: Option<u8> = None;
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None => match b {
                b'"' | b'\'' => quote = Some(b),
                b'>' => return i - start + 1,
                _ => {}
            },
        }
        i += 1;
    }
    bytes.len() - start
}

/// Offset in byte subito dopo il tag di apertura `<head …>`, se presente. Usato
/// dai fix che inseriscono un elemento come primo figlio della `<head>`.
pub fn head_open_end(dom: &VDom<'_>, parser: &Parser<'_>, src: &str) -> Option<usize> {
    let head = dom
        .query_selector("head")
        .and_then(|mut it| it.next())?
        .get(parser)?
        .as_tag()?;
    let (start, len) = opening_tag_span(head, parser, src);
    Some(start + len)
}

/// Offset in byte dove inserire un nuovo attributo (` name="v"`) nel tag di
/// apertura con span `(start, len)`: subito prima del `>` finale, oppure prima
/// del `/` se il tag è self-closing (`<img … />`).
pub fn attr_insert_pos(src: &str, span: (usize, usize)) -> usize {
    let (start, len) = span;
    let gt = start + len - 1; // posizione del '>'
    let bytes = src.as_bytes();
    if gt > start && bytes[gt - 1] == b'/' {
        gt - 1
    } else {
        gt
    }
}

/// Vero se un elemento interattivo ha un "nome accessibile": testo visibile,
/// `aria-label`, `title`, oppure un'immagine discendente con `alt` non vuoto.
pub fn has_accessible_name(tag: &HTMLTag<'_>, parser: &Parser<'_>) -> bool {
    if attr_non_empty(tag, "aria-label") || attr_non_empty(tag, "title") {
        return true;
    }
    if !tag.inner_text(parser).trim().is_empty() {
        return true;
    }
    tag.children().all(parser).iter().any(|n| {
        n.as_tag().is_some_and(|t| {
            t.name().as_bytes().eq_ignore_ascii_case(b"img") && attr_non_empty(t, "alt")
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_doc(src: &str) -> bool {
        let dom = tl::parse(src, tl::ParserOptions::default()).unwrap();
        is_full_document(&dom, src)
    }

    #[test]
    fn pagina_con_html_e_documento() {
        assert!(full_doc("<html><body><p>x</p></body></html>"));
    }

    #[test]
    fn solo_head_e_documento() {
        assert!(full_doc("<head><title>x</title></head>"));
    }

    #[test]
    fn doctype_e_documento() {
        assert!(full_doc("<!DOCTYPE html>\n<body>ciao</body>"));
    }

    #[test]
    fn frammento_non_e_documento() {
        assert!(!full_doc(r#"<div class="card"><img src="a.png"></div>"#));
        assert!(!full_doc("<li>voce</li>"));
    }
}
