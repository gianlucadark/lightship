# Lightship 🛳️

Linter statico per l'**output HTML di build** — qualunque framework (Astro, Vite,
Next, SvelteKit, Hugo, Jekyll, sito statico a mano…).

Lightship analizza i file `.html`/`.css` **già buildati**, senza avviare un
browser, e **fa fallire la build** (exit code ≠ 0) quando trova problemi di
accessibilità, SEO o performance. È pensato per girare in CI subito dopo lo step
di build, sulla cartella di output (`dist/`, `build/`, `out/`, `_site/`…).

> Il cuore è una **libreria Rust pura** (`lightship-core`); la CLI (`lightship`)
> e i futuri binding npm sono gusci sottili sopra di essa.

---

## Cosa controlla

Ogni problema trovato è un **finding** con una gravità:

- **Error** → fa uscire il processo con codice `1` (rompe la build in CI).
- **Warn** → segnalato ma non blocca (exit `0` se non ci sono Error).

| Regola             | Gravità | Cosa verifica |
|--------------------|---------|---------------|
| `img-alt`          | Error   | Ogni `<img>` ha l'attributo `alt` (`alt=""` è valido per le decorative). |
| `html-lang`        | Error   | `<html>` ha un `lang` non vuoto. |
| `title-present`    | Error   | Esiste un `<title>` con testo non vuoto. |
| `duplicate-id`     | Error   | Gli `id` sono unici nel documento. |
| `meta-charset`     | Warn    | È presente `<meta charset>`. |
| `meta-viewport`    | Warn    | È presente `<meta name="viewport">` (responsive). |
| `meta-description` | Warn    | È presente `<meta name="description">` con `content` non vuoto. |
| `img-dimensions`   | Warn    | Ogni `<img>` ha `width` **e** `height` (evita layout shift / CLS). |
| `a-no-text`        | Warn    | Ogni `<a href>` ha un nome accessibile (testo, `aria-label`, `title`, o `<img alt>`). |

---

## Come funziona (architettura)

```
                 ┌────────────────────────── lightship-core (libreria pura) ──────────────────────────┐
  cartella  ──►  discovery (ignore)  ──►  parse HTML (tl)  ──►  regole (rayon, in parallelo)  ──►  Finding[]
                                                                              │
   CLI (lightship) ◄── exit 0/1 ── riepilogo + report leggibile (miette) ◄────┘
```

1. **Discovery** — con il crate [`ignore`](https://docs.rs/ignore) cammina
   ricorsivamente la cartella e raccoglie tutti i `.html`. I filtri `.gitignore`
   sono **disattivati** di proposito: l'output di build spesso vive in cartelle
   gitignorate (`dist/`), e vanno comunque analizzate. Le cartelle nascoste
   (`.git`…) vengono saltate.
2. **Parse** — ogni file è parsato in un DOM interrogabile con
   [`tl`](https://docs.rs/tl), un parser HTML che **conserva gli offset** del
   sorgente. I file sono indipendenti, quindi vengono processati **in parallelo**
   con [`rayon`](https://docs.rs/rayon).
3. **Regole** — ogni regola implementa il trait
   `Rule { id(); check(&VDom, &str) -> Vec<Finding> }` e vive nel suo modulo
   (`crates/core/src/rules/`). Aggiungerne una è banale: nuovo file + una riga in
   `rules::all()`.
4. **Report** — i finding sono raggruppati per file e renderizzati con
   [`miette`](https://docs.rs/miette) (feature `fancy`): regola, gravità,
   messaggio e lo **snippet di codice reale** con riga/colonna esatte. Chiude con
   un riepilogo `N pagine · Xms · Y error, Z warn`.

### Snippet di codice reale

Ogni `Finding` legato a un elemento porta lo **span** `(offset, len)` del tag di
apertura nel sorgente: il report estrae da lì il **codice vero** del file (non una
ricostruzione) e miette ci stampa sopra **numero di riga e colonna corretti** più
le righe di contesto. I finding "di documento" (es. `<meta>` mancante) non hanno
un elemento, quindi mostrano il solo messaggio. Questo è possibile perché `tl`
espone gli offset di sorgente, che `scraper`/html5ever invece normalizzavano via.

---

## Installazione

Serve [Rust](https://rustup.rs) (edition 2024, testato con Rust 1.96).

```bash
# dalla root del repo: installa il binario `lightship` a livello utente
cargo install --path crates/cli
```

Ora `lightship` è nel PATH (`~/.cargo/bin`). In alternativa, senza installare,
puoi sempre usare `cargo run -p lightship -- <cartella>` dalla root del repo.

---

## Uso

```bash
lightship [OPZIONI] [CARTELLA]
```

- `CARTELLA` — cartella da analizzare (default: cartella corrente).
- `-q, --quiet` — stampa solo la riga di riepilogo.
- `-h, --help` — aiuto.
- `-V, --version` — versione.

Exit code: `0` se non ci sono Error, `1` se almeno un finding è Error.

---

## Testarlo su un TUO progetto (es. Astro / Vite / Next)

L'idea: **builda il tuo progetto** come fai di solito, poi punta Lightship alla
cartella di output.

```bash
# 1. nel tuo progetto, builda come al solito
npm run build        # Astro/Vite → dist/ , Next export → out/ , Eleventy → _site/ ...

# 2. lancia Lightship sulla cartella di output
lightship dist
```

Esempio con output reale:

```text
━━ dist/blog/index.html (2 finding) ━━

img-alt

  x <img> senza attributo alt
   ,-[dist/blog/index.html:1:1]
 1 | <img src="/assets/cover.png">
   : ^^^^^^^^^^^^^|^^^^^^^^^^^^^^^
   :              `-- qui
   `----

...

12 pagine · 7ms · 3 error, 5 warn
```

Se non vuoi installare il binario, dalla root di **questo** repo puoi puntare a
una cartella qualsiasi del tuo PC (anche fuori dal repo):

```bash
cargo run -p lightship -- "C:/percorso/al/tuo/progetto/dist"
```

### In CI (blocca la build sugli Error)

```yaml
# esempio GitHub Actions
- run: npm run build
- run: lightship dist     # esce con 1 se ci sono Error → il job fallisce
```

---

## Sviluppo

```bash
cargo build      # compila core + CLI
cargo test       # unit test per regola + test di integrazione sulle fixture
cargo run -p lightship -- crates/core/tests/fixtures   # demo sulle fixture
```

Struttura:

```
crates/
  core/                 # libreria pura: tutta la logica
    src/
      lib.rs            # run / run_with / analyze (orchestrazione)
      finding.rs        # Finding, Severity
      rule.rs           # trait Rule
      report.rs         # rendering miette
      util.rs           # helper (span del tag, nome accessibile)
      rules/            # una regola = un modulo
    tests/
      fixtures/         # pagine HTML di esempio (una violazione ciascuna)
      integration.rs    # conteggi attesi
  cli/                  # binario `lightship`: parsing argomenti + exit code
```

### Aggiungere una regola

1. Crea `crates/core/src/rules/la_mia_regola.rs` con una struct che implementa
   `Rule`.
2. Aggiungi `mod la_mia_regola;` e una riga in `rules::all()`.
3. Aggiungi una fixture e un assert nel test di integrazione.

---

## Fuori scope (per ora)

Volutamente non ancora implementati, per non bloccare scelte architetturali:
scoring/punteggi, file di config (`lightship.toml`), budget, output JSON/SARIF,
binding `napi-rs` / pacchetto npm, plugin Vite/Astro. Arriveranno nelle fasi
successive.
