# lightship-cli 🛳️

Static linter for **built HTML output** — any framework (Astro, Vite, Next,
SvelteKit, Hugo, Jekyll, hand-written static sites…).

Lightship analyzes the **already-built** `.html`/`.css` files, without launching a
browser, and **fails your build** (non-zero exit code) when it finds
accessibility, SEO or performance problems. Meant to run in CI right after your
build step, pointed at the output folder (`dist/`, `build/`, `out/`, `_site/`…).

Powered by a Rust core; this npm package ships a prebuilt binary for your
platform — **no Rust toolchain required**.

## Install

```bash
npm install -g lightship-cli
# or, one-off:
npx lightship-cli dist
```

## Usage

```bash
lightship [OPTIONS] [FOLDER]
```

- `FOLDER` — folder to scan recursively (default: current folder).
- `-q, --quiet` — print only the summary line.
- `-h, --help`, `-V, --version`.

Exit code: `0` if no errors, `1` if at least one finding has severity Error.

### In CI

```yaml
- run: npm run build
- run: npx lightship-cli dist   # exits 1 on Error → the job fails
```

## What it checks

| Rule | Severity | Checks |
|------|----------|--------|
| `img-alt` | Error | every `<img>` has an `alt` attribute |
| `html-lang` | Error | `<html>` has a non-empty `lang` |
| `title-present` | Error | a non-empty `<title>` exists |
| `duplicate-id` | Error | `id`s are unique |
| `meta-charset` | Warn | `<meta charset>` is present |
| `meta-viewport` | Warn | responsive `<meta name="viewport">` is present |
| `meta-description` | Warn | non-empty `<meta name="description">` |
| `img-dimensions` | Warn | every `<img>` has `width` **and** `height` (avoids CLS) |
| `a-no-text` | Warn | every `<a href>` has an accessible name |

## Supported platforms

Windows / macOS / Linux, on x64 and arm64. The right prebuilt binary is pulled
in automatically via npm `optionalDependencies`.

## License

MIT
