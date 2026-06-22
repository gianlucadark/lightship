# Pubblicare su npm

La CLI è scritta in Rust e distribuita su npm come **binari precompilati**, con
lo schema "launcher + pacchetti per-piattaforma" (lo stesso di esbuild/biome):

- `lightship-cli` — pacchetto launcher (questo è quello che gli utenti
  installano). Contiene solo `bin.js`, che trova ed esegue il binario giusto.
- `lightship-cli-<os>-<cpu>` — un pacchetto per piattaforma, ognuno con dentro
  il binario nativo e i campi `os`/`cpu`. Sono dichiarati come
  `optionalDependencies` del launcher, quindi npm scarica solo quello giusto.

Piattaforme prodotte: `win32-x64`, `win32-arm64`, `darwin-x64`, `darwin-arm64`,
`linux-x64`, `linux-arm64`.

## Setup una tantum

1. Crea l'account/organizzazione su npm e verifica che i nomi siano liberi
   (`lightship-cli` e `lightship-cli-*`).
2. Crea un **Automation token** su npm (Account → Access Tokens → Granular o
   Automation) con permesso di publish.
3. Su GitHub: repo → Settings → Secrets and variables → Actions → aggiungi il
   secret `NPM_TOKEN` con quel token.
4. Assicurati che il repo GitHub sia `gianlucadark/lightship` (oppure aggiorna
   gli URL in `crates/*/Cargo.toml`, `npm/lightship-cli/package.json` e
   `scripts/build-npm.mjs`).

## Rilasciare una versione

La versione è guidata dal **tag git** `vX.Y.Z`. Tieni allineate le versioni in
`Cargo.toml` (è buona norma) e poi:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Il workflow `.github/workflows/release.yml`:

1. compila il binario per ogni piattaforma,
2. assembla e pubblica ogni pacchetto `lightship-cli-<os>-<cpu>`,
3. pubblica per ultimo il launcher `lightship-cli`.

## Provare in locale (solo la tua piattaforma)

```bash
# 1. compila per il tuo target nativo
cargo build --release --target x86_64-pc-windows-msvc

# 2. genera il pacchetto per-piattaforma in npm/dist/
node scripts/build-npm.mjs platform x86_64-pc-windows-msvc 0.1.0

# 3. allinea la versione del launcher
node scripts/build-npm.mjs prepare 0.1.0

# 4. testa l'installazione in locale, FEDELE al registry, via tarball.
#    (NON usare `npm install ./percorso`: i path locali vengono symlinkati e il
#     launcher non riesce a risolvere il pacchetto per-piattaforma — con un
#     install vero dal registry sono cartelle reali affiancate e funziona.)
npm pack ./npm/dist/lightship-cli-win32-x64
npm pack ./npm/lightship-cli
mkdir /tmp/lstest && cd /tmp/lstest && npm init -y
npm install /percorso/lightship-cli-win32-x64-0.1.0.tgz /percorso/lightship-cli-0.1.0.tgz
npx lightship --version
```

## Note

- Il binario per Linux si compila in versione **glibc** (gnu). Per Alpine/musl
  servirebbe aggiungere i target `*-unknown-linux-musl` alla matrice e a
  `scripts/build-npm.mjs` + `npm/lightship-cli/bin.js`.
- Per abilitare la **provenance** npm (badge di build verificata), serve repo
  pubblico: aggiungi `permissions: id-token: write` ai job di publish e
  `--provenance` ai comandi `npm publish`.
