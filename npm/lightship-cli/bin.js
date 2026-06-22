#!/usr/bin/env node
"use strict";

// Launcher: trova il binario `lightship` precompilato per la piattaforma
// corrente (installato come optionalDependency) e lo esegue passando gli
// argomenti così come sono. L'exit code del binario diventa il nostro.

const { spawnSync } = require("node:child_process");

// piattaforma+arch -> nome del pacchetto per-piattaforma.
// Deve restare allineato a scripts/build-npm.mjs e agli optionalDependencies
// in package.json.
const PLATFORM_PACKAGES = {
  "win32 x64": "@gianluca.darcangelo/lightship-cli-win32-x64",
  "win32 arm64": "@gianluca.darcangelo/lightship-cli-win32-arm64",
  "darwin x64": "@gianluca.darcangelo/lightship-cli-darwin-x64",
  "darwin arm64": "@gianluca.darcangelo/lightship-cli-darwin-arm64",
  "linux x64": "@gianluca.darcangelo/lightship-cli-linux-x64",
  "linux arm64": "@gianluca.darcangelo/lightship-cli-linux-arm64",
};

function binaryPath() {
  const key = `${process.platform} ${process.arch}`;
  const pkg = PLATFORM_PACKAGES[key];
  if (!pkg) {
    throw new Error(
      `lightship: piattaforma non supportata (${key}).\n` +
        `Piattaforme disponibili: ${Object.keys(PLATFORM_PACKAGES).join(", ")}.`
    );
  }
  const exe = process.platform === "win32" ? "lightship.exe" : "lightship";
  try {
    return require.resolve(`${pkg}/bin/${exe}`);
  } catch {
    throw new Error(
      `lightship: binario non trovato per ${key} (pacchetto "${pkg}" mancante).\n` +
        `Prova a reinstallare: npm install lightship-cli\n` +
        `Se usi "--no-optional", un mirror o un lockfile parziale, gli\n` +
        `optionalDependencies potrebbero essere stati saltati.`
    );
  }
}

function main() {
  let bin;
  try {
    bin = binaryPath();
  } catch (err) {
    console.error(err.message);
    process.exit(1);
  }

  const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });

  if (result.error) {
    console.error(`lightship: impossibile eseguire il binario: ${result.error.message}`);
    process.exit(1);
  }
  // Se ucciso da un segnale, status è null: usciamo comunque con errore.
  process.exit(result.status === null ? 1 : result.status);
}

main();
