#!/usr/bin/env node
// Genera i pacchetti npm per la distribuzione dei binari precompilati.
//
// Uso:
//   node scripts/build-npm.mjs prepare  <version>
//       Allinea la versione del pacchetto launcher (npm/lightship-cli) e dei
//       suoi optionalDependencies a <version>.
//
//   node scripts/build-npm.mjs platform <rust-target> <version>
//       Crea npm/dist/<pkg>/ con dentro il binario compilato in
//       target/<rust-target>/release/ e un package.json con os/cpu corretti.
//
// Pensato per essere chiamato dalla CI (vedi .github/workflows/release.yml),
// ma eseguibile anche in locale.

import { mkdirSync, copyFileSync, writeFileSync, readFileSync, existsSync, chmodSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

// rust target triple -> metadati npm. Allineato a npm/lightship-cli/bin.js.
const TARGETS = {
  "x86_64-pc-windows-msvc": { os: "win32", cpu: "x64", bin: "lightship.exe" },
  "aarch64-pc-windows-msvc": { os: "win32", cpu: "arm64", bin: "lightship.exe" },
  "x86_64-apple-darwin": { os: "darwin", cpu: "x64", bin: "lightship" },
  "aarch64-apple-darwin": { os: "darwin", cpu: "arm64", bin: "lightship" },
  "x86_64-unknown-linux-gnu": { os: "linux", cpu: "x64", bin: "lightship" },
  "aarch64-unknown-linux-gnu": { os: "linux", cpu: "arm64", bin: "lightship" },
};

const pkgName = (os, cpu) => `lightship-cli-${os}-${cpu}`;

function readJson(p) {
  return JSON.parse(readFileSync(p, "utf8"));
}
function writeJson(p, obj) {
  writeFileSync(p, JSON.stringify(obj, null, 2) + "\n");
}

function prepare(version) {
  const mainPkgPath = join(ROOT, "npm", "lightship-cli", "package.json");
  const pkg = readJson(mainPkgPath);
  pkg.version = version;
  for (const key of Object.keys(pkg.optionalDependencies ?? {})) {
    pkg.optionalDependencies[key] = version;
  }
  writeJson(mainPkgPath, pkg);
  console.log(`prepared launcher package at version ${version}`);
}

function platform(target, version) {
  const meta = TARGETS[target];
  if (!meta) {
    throw new Error(`target non riconosciuto: ${target}\nDisponibili: ${Object.keys(TARGETS).join(", ")}`);
  }
  const { os, cpu, bin } = meta;
  const name = pkgName(os, cpu);

  const builtBinary = join(ROOT, "target", target, "release", bin);
  if (!existsSync(builtBinary)) {
    throw new Error(`binario non trovato: ${builtBinary}\n(esegui prima: cargo build --release --target ${target})`);
  }

  const outDir = join(ROOT, "npm", "dist", name);
  const binDir = join(outDir, "bin");
  mkdirSync(binDir, { recursive: true });

  const destBinary = join(binDir, bin);
  copyFileSync(builtBinary, destBinary);
  if (os !== "win32") {
    chmodSync(destBinary, 0o755);
  }

  writeJson(join(outDir, "package.json"), {
    name,
    version,
    description: `lightship prebuilt binary for ${os} ${cpu}`,
    license: "MIT",
    repository: {
      type: "git",
      url: "git+https://github.com/gianlucadark1/lightship.git",
    },
    os: [os],
    cpu: [cpu],
    files: ["bin/"],
  });

  console.log(`built ${name}@${version} -> ${outDir}`);
  return outDir;
}

// Stampa la cartella del pacchetto per-piattaforma di un target (senza
// compilare nulla). Comodo in CI per sapere cosa pubblicare.
function pkgdir(target) {
  const meta = TARGETS[target];
  if (!meta) {
    throw new Error(`target non riconosciuto: ${target}`);
  }
  process.stdout.write(join("npm", "dist", pkgName(meta.os, meta.cpu)));
}

function main() {
  const [, , cmd, ...rest] = process.argv;
  if (cmd === "prepare") {
    const version = rest[0];
    if (!version) throw new Error("uso: build-npm.mjs prepare <version>");
    prepare(version);
  } else if (cmd === "platform") {
    const [target, version] = rest;
    if (!target || !version) throw new Error("uso: build-npm.mjs platform <rust-target> <version>");
    platform(target, version);
  } else if (cmd === "pkgdir") {
    const target = rest[0];
    if (!target) throw new Error("uso: build-npm.mjs pkgdir <rust-target>");
    pkgdir(target);
  } else {
    throw new Error("comando sconosciuto. Usa 'prepare' o 'platform'.");
  }
}

try {
  main();
} catch (err) {
  console.error(String(err.message ?? err));
  process.exit(1);
}
