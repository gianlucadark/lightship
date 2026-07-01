# Lightship 🛳️

A static linter for your **built HTML output** — any framework (Astro, Vite,
Next, SvelteKit, Hugo, Jekyll, a hand-written static site…).

Lightship analyzes your **already-built** `.html` files, without spinning up a
browser, and **fails the build** (non-zero exit code) when it finds accessibility,
SEO or performance problems. It's designed to run in CI right after the build
step, on the output folder (`dist/`, `build/`, `out/`, `_site/`…).

> The core is a **pure Rust library** (`lightship-core`); the CLI (`lightship`)
> and future npm bindings are thin shells on top of it.

---

## What it checks

Every problem is a **finding** with a severity:

- **Error** → exits with code `1` (breaks the CI build).
- **Warn** → reported but non-blocking (exit `0` if there are no errors).

Rules are grouped into **categories** — accessibility, SEO, performance, security,
correctness — which you can filter with `--only-category` and `--preset`.
Run `lightship rules` for the full, always-up-to-date list, or
`lightship explain <rule>` for details on any one. The current set (26 rules):

### Accessibility
| Rule | Severity | Checks |
|------|----------|--------|
| `img-alt`           | Error | Every `<img>` has an `alt` (`alt=""` is valid for decorative images). |
| `html-lang`         | Error | `<html>` has a non-empty `lang`. |
| `title-present`     | Error | A `<title>` with non-empty text exists. |
| `a-no-text`         | Warn  | Every `<a href>` has an accessible name. |
| `button-name`       | Warn  | Every `<button>` has an accessible name. |
| `label-control`     | Warn  | Every form control has an associated label. |
| `label-for-target`  | Warn  | Every `<label for>` points to an existing element id. |
| `aria-ref-target`   | Warn  | `aria-labelledby`/`aria-describedby` reference existing ids. |
| `iframe-title`      | Warn  | Every `<iframe>` has a non-empty `title`. |
| `heading-order`     | Warn  | Heading levels increase by one, without skipping. |
| `positive-tabindex` | Warn  | No element uses a positive `tabindex`. |
| `table-headers`     | Warn  | Data tables have header cells (`<th>`). |
| `lang-valid`        | Warn  | The `<html lang>` value is a well-formed BCP-47 tag. |

### SEO
| Rule | Severity | Checks |
|------|----------|--------|
| `meta-charset`            | Warn | `<meta charset>` is present. |
| `meta-viewport`           | Warn | `<meta name="viewport">` is present (responsive). |
| `meta-description`        | Warn | `<meta name="description">` with non-empty `content`. |
| `meta-description-length` | Warn | The meta description is ~50–160 characters. |
| `title-length`            | Warn | The `<title>` is at most ~60 characters. |
| `single-h1`               | Warn | The page has exactly one `<h1>`. |
| `canonical-link`          | Warn | At most one `<link rel="canonical">` with a non-empty href. |
| `duplicate-meta`          | Warn | Single-instance head tags (title/charset/viewport/description) aren't duplicated. |

### Performance
| Rule | Severity | Checks |
|------|----------|--------|
| `img-dimensions`         | Warn | Every `<img>` has `width` **and** `height` (avoids layout shift / CLS). |
| `render-blocking-script` | Warn | No render-blocking `<script>` in `<head>`. |
| `img-lazy-loading`       | Warn | Every `<img>` declares a `loading` strategy. *(opt-in: `--preset all`)* |

### Security
| Rule | Severity | Checks |
|------|----------|--------|
| `link-target-blank` | Warn | `<a target="_blank">` sets `rel="noopener"`. |

### Correctness
| Rule | Severity | Checks |
|------|----------|--------|
| `duplicate-id` | Error | Element ids are unique in the document. |

### Fragments vs. full pages

Many build outputs contain **HTML fragments** — component partials, email
templates, htmx/Turbo snippets — that have no `<html>`/`<head>`. Document-level
rules (title, charset, viewport, single-h1, heading-order, …) only run on **full
pages** so those fragments don't produce false "missing title/charset/viewport"
findings. Element-level rules (`img-alt`, `a-no-text`, …) still run everywhere.
Pass `--include-fragments` (or `[analyze] include_fragments = true`) to run
everything on every file.

---

## How it works (architecture)

```
                 ┌──────────────────── lightship-core (pure library) ────────────────────┐
  folder   ──►   discovery (ignore)  ──►  parse HTML (tl)  ──►  rules (rayon, in parallel)  ──►  Finding[]
                                                                          │
   CLI (lightship) ◄── exit 0/1 ── dashboard / json / sarif / github  ◄───┘
```

1. **Discovery** — walks the folder recursively with [`ignore`](https://docs.rs/ignore)
   and collects every `.html`. Gitignore filters are **disabled** on purpose: build
   output often lives in gitignored folders (`dist/`) and must still be analyzed.
   Heavy folders (`node_modules`, `vendor`, …) are pruned; hidden ones skipped.
2. **Parse** — each file is parsed into a queryable DOM with
   [`tl`](https://docs.rs/tl), an HTML parser that **keeps source offsets**. Files
   are independent, so they're processed **in parallel** with
   [`rayon`](https://docs.rs/rayon). Non-UTF-8 bytes are read leniently; unreadable,
   unparseable or oversized files are **skipped** (and counted, not silently dropped).
3. **Rules** — each rule implements `Rule { id(); meta(); scope(); check(&VDom, &str) }`
   and lives in its own module (`crates/core/src/rules/`). Adding one is trivial:
   a new file plus one line in `rules::all()`.
4. **Report** — findings are grouped by file and rendered by the `report/` module in
   several formats: `pretty` (colored dashboard with real code snippets, per-page
   score and a summary panel), `compact`, `json`, `sarif`, `github`. ANSI colors are
   handled by [`anstream`](https://docs.rs/anstream) + [`owo-colors`](https://docs.rs/owo-colors)
   and stripped automatically outside a terminal.

### Real code snippets

Every element-bound `Finding` carries the **span** `(offset, len)` of the opening
tag in the source: the report extracts the **actual code** from the file (not a
reconstruction) and computes **correct line and column**. Document-level findings
(e.g. a missing `<meta>`) have no element, so they show just the message. This is
possible because `tl` exposes source offsets, which `scraper`/html5ever normalize away.

---

## Installation

Requires [Rust](https://rustup.rs) (edition 2024).

```bash
# from the repo root: install the `lightship` binary user-wide
cargo install --path crates/cli
```

`lightship` is now on your PATH (`~/.cargo/bin`). Or, without installing, use
`cargo run -p lightship -- <folder>` from the repo root.

---

## Usage

```bash
lightship                       # auto-detect the build folder and analyze it
lightship [FOLDER]              # = lightship analyze FOLDER
lightship analyze [FOLDER]      # analyze (aliases: check, scan)
lightship rules                 # list every rule with severity and description
lightship explain <rule>        # rule detail: what it checks, how to fix, examples
lightship init [FOLDER]         # create a lightship.toml (detects your framework)
lightship ci [FOLDER]           # scaffold a ready-to-use GitHub Actions workflow
lightship baseline [FOLDER]     # freeze current findings so CI only fails on new ones
lightship fix [FOLDER]          # interactively apply safe automatic fixes
```

> **Zero-config:** run with **no arguments** and Lightship tries to detect your
> project's output folder (from the framework in `package.json` or config files;
> falling back to `dist`, `build`, `out`, `_site`, `public`) and analyze it. The
> detection note goes to **stderr**, so `--format json|sarif|github` stays clean on
> stdout.

### `analyze` options

- `FOLDER` — folder to analyze (default: auto-detected).
- `-q, --quiet` — print only the summary panel.
- `-v, --verbose` — more detailed output (no snippet truncation).
- `--format <pretty|compact|json|sarif|github>` — output format (default `pretty`).
- `--only <rules>` — run only these rules (comma-separated).
- `--only-category <cats>` — run only these categories (`accessibility`/`a11y`, `seo`, `performance`, `security`, `correctness`).
- `--preset <recommended|all|CATEGORY>` — rule set to run (default `recommended`).
- `--include-fragments` — also run document-level rules on partials/fragments.
- `--baseline <path>` — suppress known findings from a baseline file.
- `--max-warnings <N>` — fail the build if warnings exceed `N`.
- `--error-on-warnings` — fail on any warning.
- `--color <auto|always|never>` / `--no-color` — control ANSI colors.
- `--ascii` — ASCII-only glyphs (for terminals that don't render box-drawing).
- `--no-suggestions` — hide the 💡 fix line.
- `--config <path>` — explicit config file.
- `--watch` — re-analyze automatically on file changes.

Exit code: `0` if there are no errors (and warnings are within your thresholds), `1` otherwise.

### Output

The default `pretty` format is a **colored dashboard**: banner, findings grouped by
file with real code snippets (line/column + underline), a **fix suggestion** for
each, a per-file grade, and a summary panel with a per-rule chart, a **0–100 score
(grade A–F)** and a `PASS`/`FAIL` verdict. Colors turn off automatically when the
output isn't a terminal (handy in CI) or with `--no-color`/`NO_COLOR`.

For CI and code scanning use `--format json`, `--format sarif` (SARIF 2.1.0 with
`endLine`/`endColumn` regions and stable `partialFingerprints`), or `--format github`
(inline annotations). JSON includes per-finding `category`/`docs_url` and per-page scores.

### Baseline (adopt on an existing project)

Introducing a linter on a mature site can surface dozens of pre-existing issues.
A **baseline** freezes today's state so CI only fails on **new** problems:

```bash
lightship baseline dist     # writes dist/lightship-baseline.json
lightship dist              # future runs suppress the frozen findings
```

Suppression is occurrence-accurate: if a page had one `img-alt` and now has two,
the baseline covers one and the **new** one still fails the build. The baseline file
is human-readable (rule + file + message) and safe to review in a PR.

### Autofix (`lightship fix`)

Some problems have a single, unambiguous correction, so Lightship can apply it for
you. `lightship fix` lists every **safe, deterministic** fix — numbered, with the
exact change it will make — and lets you choose which to apply (individually, as a
range, or all):

```text
5 auto-fixable issue(s):

  [1] add rel="noopener" to this target="_blank" link  (link-target-blank)
       dist/page.html:1:85
       → insert: rel="noopener"
  [2] insert <meta charset="utf-8"> as the first <head> child  (meta-charset)
       ...

Select fixes to apply [e.g. 1,3 or 2-4 · 'a' all · 'q' quit]:
```

- `--all` applies every fix without prompting; `--dry-run` previews without writing.
- Fixable rules: `link-target-blank`, `img-lazy-loading`, `meta-charset`,
  `meta-viewport`, `html-lang` (inserts `lang="en"` — change it if the page isn't
  English). Rules that need human judgment (`img-alt` text, `img-dimensions`) are
  reported but never auto-changed.

> Note: Lightship fixes the **built output**, which is regenerated on the next build.
> Use `fix` to preview/patch a one-off artifact; for lasting fixes, apply the change
> in your source templates.

### Configuration (`lightship.toml`)

`lightship init` creates an example file. It lets you override severity or disable
rules, exclude paths, pick a default format/preset, and set CI thresholds:

```toml
[rules]
img-alt = "error"          # error | warn | off
meta-viewport = "off"

[ignore]
paths = ["**/404.html", "**/_*.html"]

[analyze]
# include_fragments = false     # run document rules on partials too
# max_file_bytes = 8388608      # skip files larger than this
# preset = "recommended"        # recommended | all | <category>

[output]
format = "pretty"               # pretty | compact | json | sarif | github

[ci]
# max_warnings = 0              # fail if warnings exceed this
# error_on_warnings = false     # fail on any warning
```

The file is looked up via `--config`, then the analyzed folder, then the cwd.

---

## Try it on YOUR project (Astro / Vite / Next / …)

The idea: **build your project** as usual, then point Lightship at the output folder.

```bash
# 1. in your project, build as usual
npm run build        # Astro/Vite → dist/ , Next export → out/ , Eleventy → _site/ ...

# 2. run Lightship on the output folder
lightship dist
```

Example real output (`pretty`):

```text
◆  LIGHTSHIP v1.0.0
   Scanning dist

   CHECK FAILED  1 error · 0 warnings · 12 pages

┌─ dist/blog/index.html  1 error · F 90
│
├─  ERROR    img-alt · 1:1
│     <img> is missing an alt attribute
│     1 │ <img src="/assets/cover.png">
│       │ ▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲ here
│     Fix Add a descriptive alt; use alt="" for purely decorative images.
└─

╭─ Results ─────────────────────────────────────────────────────
│  FAIL  1 error  0 warnings
│  Health  ■■■■■■■■■·  90/100 · grade A
│  Pages 12  Time 7 ms
├─ Issues by rule ──────────────────────────────────────────────
│  ● img-alt   1
│
│  Start here img-alt (1) → lightship explain img-alt
╰──────────────────────────────────────────────────────────────
```

### In CI (fail the build on errors)

The quickest way: `lightship ci` scaffolds `.github/workflows/lightship.yml`
(build + analyze the detected folder) without overwriting an existing workflow.
Or add it by hand:

```yaml
# GitHub Actions example
- run: npm run build
- run: npx lightship dist   # exits 1 on errors → the job fails
```

For GitHub code scanning, upload the SARIF:

```yaml
- run: npx lightship dist --format sarif > lightship.sarif
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: lightship.sarif
```

---

## Development

```bash
cargo build      # build core + CLI
cargo test       # per-rule unit tests + integration tests on fixtures
cargo run -p lightship -- crates/core/tests/fixtures   # demo on the fixtures
```

Layout:

```
crates/
  core/                 # pure library: all the logic
    src/
      lib.rs            # run / run_with / analyze (orchestration)
      finding.rs        # Finding, Severity, line/col, fingerprint
      rule.rs           # Rule trait (id + meta + scope + check)
      meta.rs           # RuleMeta + Category
      config.rs         # lightship.toml (severity/ignore/analyze/ci)
      baseline.rs       # baseline / suppression
      detect.rs         # framework & output-folder auto-detection
      report/           # rendering: pretty, compact, json, sarif, github
      util.rs           # helpers (tag span, accessible name, ids, is_full_document)
      rules/            # one rule = one module
    tests/
      fixtures/         # sample HTML pages (one violation each)
      integration.rs    # expected counts, fragments, presets, baseline
  cli/                  # the `lightship` binary: arg parsing + exit code
```

### Adding a rule

1. Create `crates/core/src/rules/my_rule.rs` with a struct implementing `Rule`
   (`id` + `meta` with category/help/examples + `check`; override `scope` for
   document-level rules).
2. Add `mod my_rule;` and one line in `rules::all()`.
3. Add a unit test, and (optionally) a fixture + assertion in the integration test.

---

## Out of scope (for now)

Deliberately not implemented yet: CSS rules, performance budgets, native
`napi-rs` bindings, and Vite/Astro plugins. Coming in later phases.
