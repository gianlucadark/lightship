use std::borrow::Cow;
use tl::{HTMLTag, Parser};

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
    tag.attributes().iter().any(|(k, _)| k.eq_ignore_ascii_case(name))
}

/// Vero se l'attributo (case-insensitive) esiste con un valore non vuoto.
pub fn attr_non_empty(tag: &HTMLTag<'_>, name: &str) -> bool {
    attr(tag, name).is_some_and(|v| !v.trim().is_empty())
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
        n.as_tag()
            .is_some_and(|t| t.name().as_bytes().eq_ignore_ascii_case(b"img") && attr_non_empty(t, "alt"))
    })
}
