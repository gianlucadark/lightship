//! Feedback leggero "in corso" durante l'analisi.
//!
//! Scrive **solo su stderr**, così stdout resta pulito per i formati macchina
//! (json/sarif/github) e per le pipe. Si attiva solo quando stderr è un vero
//! terminale: in CI o quando l'output è rediretto resta del tutto silenzioso e
//! a costo zero.

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Contatore di avanzamento condiviso fra i thread di rayon.
///
/// Quando `active` è `false` ogni operazione è un no-op: nessuna scrittura,
/// nessun lock, niente overhead nel caso comune (CI / output rediretto).
pub struct Progress {
    done: AtomicUsize,
    total: usize,
    active: bool,
}

impl Progress {
    /// Crea un indicatore per `total` file. Disegna solo se `enabled` (formato
    /// umano, non `--quiet`) **e** stderr è un terminale **e** ci sono abbastanza
    /// file da rendere utile il feedback.
    pub fn new(total: usize, enabled: bool) -> Self {
        let active = enabled && total > 1 && std::io::stderr().is_terminal();
        Progress {
            done: AtomicUsize::new(0),
            total,
            active,
        }
    }

    /// Segnala che un file è stato analizzato e ridisegna la riga di stato.
    pub fn tick(&self) {
        if !self.active {
            return;
        }
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        let mut err = std::io::stderr().lock();
        let _ = write!(err, "\r  Scanning {done}/{} pages…", self.total);
        let _ = err.flush();
    }

    /// Cancella la riga di stato (chiamato a fine analisi, prima del report).
    pub fn finish(&self) {
        if !self.active {
            return;
        }
        let mut err = std::io::stderr().lock();
        // Sovrascrive la riga con spazi e riporta il cursore a inizio riga.
        let _ = write!(err, "\r{}\r", " ".repeat(40));
        let _ = err.flush();
    }
}
