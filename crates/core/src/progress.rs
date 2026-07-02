//! Feedback leggero "in corso" durante l'analisi.
//!
//! Scrive **solo su stderr**, così stdout resta pulito per i formati macchina
//! (json/sarif/github) e per le pipe. Si attiva solo quando stderr è un vero
//! terminale: in CI o quando l'output è rediretto resta del tutto silenzioso e
//! a costo zero.

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Frame dello spinner (braille) per i terminali unicode.
const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
/// Frame dello spinner per `--ascii`.
const FRAMES_ASCII: &[&str] = &["|", "/", "-", "\\"];
/// Larghezza (abbondante) della riga di stato da cancellare.
const LINE_WIDTH: usize = 60;

/// Mostra `Discovering pages…` su stderr mentre gira la walk delle cartelle
/// (che su alberi grossi può richiedere un momento ed era silenziosa). La riga
/// viene poi sovrascritta dall'indicatore di scansione o cancellata da
/// [`Progress::new`].
pub fn discovering(enabled: bool, ascii: bool) {
    if !enabled || !std::io::stderr().is_terminal() {
        return;
    }
    let dots = if ascii { "..." } else { "…" };
    let mut err = std::io::stderr().lock();
    let _ = write!(err, "\r  Discovering pages{dots}");
    let _ = err.flush();
}

/// Contatore di avanzamento condiviso fra i thread di rayon.
///
/// Quando `active` è `false` ogni operazione è un no-op: nessuna scrittura,
/// nessun lock, niente overhead nel caso comune (CI / output rediretto).
pub struct Progress {
    done: AtomicUsize,
    total: usize,
    active: bool,
    start: Instant,
    frames: &'static [&'static str],
}

impl Progress {
    /// Crea un indicatore per `total` file. Disegna solo se `enabled` (formato
    /// umano, non `--quiet`) **e** stderr è un terminale **e** ci sono abbastanza
    /// file da rendere utile il feedback. Cancella comunque l'eventuale riga di
    /// [`discovering`], che altrimenti resterebbe appesa.
    pub fn new(total: usize, enabled: bool, ascii: bool) -> Self {
        let tty = enabled && std::io::stderr().is_terminal();
        if tty {
            clear_line();
        }
        Progress {
            done: AtomicUsize::new(0),
            total,
            active: tty && total > 1,
            start: Instant::now(),
            frames: if ascii { FRAMES_ASCII } else { FRAMES },
        }
    }

    /// Segnala che un file è stato analizzato e ridisegna la riga di stato
    /// (spinner + contatore + tempo trascorso).
    pub fn tick(&self) {
        if !self.active {
            return;
        }
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        let frame = self.frames[done % self.frames.len()];
        let secs = self.start.elapsed().as_secs_f32();
        let mut err = std::io::stderr().lock();
        let _ = write!(
            err,
            "\r{frame} Scanning {done}/{} pages · {secs:.1}s",
            self.total
        );
        let _ = err.flush();
    }

    /// Cancella la riga di stato (chiamato a fine analisi, prima del report).
    pub fn finish(&self) {
        if !self.active {
            return;
        }
        clear_line();
    }
}

/// Sovrascrive la riga di stato con spazi e riporta il cursore a inizio riga.
fn clear_line() {
    let mut err = std::io::stderr().lock();
    let _ = write!(err, "\r{}\r", " ".repeat(LINE_WIDTH));
    let _ = err.flush();
}
