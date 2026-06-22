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
   CLI (lightship) ◄── exit 0/1 ── dashboard / json / sarif / github  ◄───────┘
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
4. **Report** — i finding sono raggruppati per file e renderizzati dal modulo
   `report/` in più formati: `pretty` (dashboard a colori con suggerimenti e
   pannello di riepilogo), `compact`, `json`, `sarif`, `github`. I colori ANSI
   sono gestiti da [`anstream`](https://docs.rs/anstream) + [`owo-colors`](https://docs.rs/owo-colors)
   e rimossi automaticamente fuori dal terminale.

### Snippet di codice reale

Ogni `Finding` legato a un elemento porta lo **span** `(offset, len)` del tag di
apertura nel sorgente: il report estrae da lì il **codice vero** del file (non una
ricostruzione) e calcola **numero di riga e colonna corretti**. I finding "di
documento" (es. `<meta>` mancante) non hanno un elemento, quindi mostrano il solo
messaggio. Questo è possibile perché `tl` espone gli offset di sorgente, che
`scraper`/html5ever invece normalizzavano via.

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
lightship                       # auto-rileva la cartella di build e la analizza
lightship [CARTELLA]            # = lightship analyze CARTELLA
lightship analyze [CARTELLA]    # analizza (alias: check, scan)
lightship rules                 # elenca tutte le regole con gravità e descrizione
lightship explain <regola>      # dettaglio di una regola: cosa controlla, come correggere, esempi
lightship init [CARTELLA]       # crea un lightship.toml (rileva il framework)
lightship ci [CARTELLA]         # genera un workflow GitHub Actions pronto all'uso
```

> **Zero-config:** lanciato **senza argomenti**, Lightship prova a rilevare la
> cartella di output del tuo progetto (dal framework in `package.json` o dai file
> di config; in fallback prova `dist`, `build`, `out`, `_site`, `public`) e la
> analizza. Niente da ricordare: builda e lancia `lightship`. La nota di
> rilevamento va su **stderr**, così `--format json|sarif|github` resta pulito su
> stdout.

### Opzioni di `analyze`

- `CARTELLA` — cartella da analizzare (default: auto-rilevata, vedi sopra).
- `-q, --quiet` — stampa solo il pannello di riepilogo.
- `-v, --verbose` — output più dettagliato.
- `--format <pretty|compact|json|sarif|github>` — formato di output (default `pretty`).
- `--no-color` — disattiva i colori ANSI.
- `--no-suggestions` — nasconde la riga 💡 con il fix.
- `--max-warnings <N>` — fa fallire la build se i warning superano `N`.
- `--only <regole>` — esegue solo le regole indicate (separate da virgola).
- `--config <path>` — file di config esplicito.
- `--watch` — ri-analizza automaticamente a ogni cambiamento dei file.

Exit code: `0` se non ci sono Error (e i warning sono entro `--max-warnings`), `1` altrimenti.

### Output

Il formato `pretty` (default) è una **dashboard a colori**: banner, finding raggruppati
per file con snippet di codice reale (riga/colonna + caret), un **suggerimento di fix**
per ognuno, e un pannello di riepilogo con grafico per regola, **punteggio 0–100 (voto
A–F)** e verdetto `PASS`/`FAIL`. I colori vengono disattivati automaticamente quando
l'output non è un terminale (utile in CI) o con `--no-color`/`NO_COLOR`.

Per CI e code scanning sono disponibili `--format json`, `--format sarif`
(SARIF 2.1.0, GitHub code scanning) e `--format github` (annotazioni inline).

### Configurazione (`lightship.toml`)

`lightship init` crea un file di esempio. Permette di sovrascrivere la gravità o
disattivare le regole, escludere percorsi e fissare il formato di default:

```toml
[rules]
img-alt = "error"     # error | warn | off
meta-viewport = "off"

[ignore]
paths = ["**/404.html", "**/_*.html"]

[output]
format = "pretty"
```

Il file viene cercato in `--config`, poi nella cartella analizzata, poi nella cwd.

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

Esempio con output reale (`pretty`):

L'output utente è in **inglese**:

```text
🛳  Lightship v0.2.0
   analyzing dist

❯ dist/blog/index.html  · 1 issue  ✖1 ⚠0
  ✖ img-alt  L1:C1
    <img> is missing an alt attribute
    1 │ <img src="/assets/cover.png">
      │ ▲ here
    💡 Add a descriptive alt; use alt="" for purely decorative images.

───────────────────── Summary ──────────────────────
 Pages     12           Time      7 ms
 Errors    3 ✖          Warnings  5 ⚠

 By rule
   img-alt          ███████  3
   meta-viewport    ████     2

 Score     61/100  (D)

 ✖  FAIL · 3 errors to fix
────────────────────────────────────────────────────
```

Se non vuoi installare il binario, dalla root di **questo** repo puoi puntare a
una cartella qualsiasi del tuo PC (anche fuori dal repo):

```bash
cargo run -p lightship -- "C:/percorso/al/tuo/progetto/dist"
```

### In CI (blocca la build sugli Error)

Il modo più rapido: `lightship ci` genera `.github/workflows/lightship.yml` già
pronto (build + analisi della cartella rilevata), senza sovrascrivere un workflow
esistente. In alternativa, aggiungilo a mano:

```yaml
# esempio GitHub Actions
- run: npm run build
- run: npx lightship dist   # esce con 1 se ci sono Error → il job fallisce
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
      finding.rs        # Finding, Severity, line/col
      rule.rs           # trait Rule (id + meta + check)
      meta.rs           # RuleMeta (descrizione, help, esempi)
      config.rs         # lightship.toml (off/severity/ignore)
      report/           # rendering: pretty, compact, json, sarif, github
      util.rs           # helper (span del tag, nome accessibile)
      rules/            # una regola = un modulo
    tests/
      fixtures/         # pagine HTML di esempio (una violazione ciascuna)
      integration.rs    # conteggi attesi
  cli/                  # binario `lightship`: parsing argomenti + exit code
```

### Aggiungere una regola

1. Crea `crates/core/src/rules/la_mia_regola.rs` con una struct che implementa
   `Rule` (`id` + `meta` con descrizione/help/esempi + `check`).
2. Aggiungi `mod la_mia_regola;` e una riga in `rules::all()`.
3. Aggiungi una fixture e un assert nel test di integrazione.

---

## Fuori scope (per ora)

Volutamente non ancora implementati: regole su CSS, budget di performance,
binding `napi-rs` nativi e plugin Vite/Astro. Arriveranno nelle fasi successive.
