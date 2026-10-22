#!/usr/bin/env node
// Local release pipeline for the Windows (Tauri) build.
//
//   pnpm release 0.1.2 ["release notes"]
//
// What it does:
//   1. Bumps the version in package.json and src-tauri/tauri.conf.json
//   2. Runs `pnpm build` with TAURI_SIGNING_PRIVATE_KEY so the NSIS
//      installer and its .sig updater artifact are produced
//   3. Stages everything under release/v<version>/ and generates the
//      latest.json consumed by the in-app updater
//   4. Prints the manual steps to publish on GitHub (no gh CLI needed)
//
// The updater endpoint is
//   https://github.com/<repo>/releases/latest/download/latest.json
// so the release MUST be published (not draft, not prerelease) for the
// running app to see it.

import { execFileSync, execSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const ROOT = path.resolve(import.meta.dirname, '..')
const KEY_PATH = path.join(ROOT, 'src-tauri', 'keys', 'updater.key')

function fail(msg) {
    console.error(`✖ ${msg}`)
    process.exit(1)
}

function run(cmd, args, opts = {}) {
    // Join into one command string: an args array combined with shell:true
    // is deprecated by Node (arguments are concatenated without escaping).
    execSync([cmd, ...args].join(' '), { stdio: 'inherit', ...opts })
}

// ---------- inputs ----------
const version = process.argv[2]
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
    fail('usage: pnpm release <x.y.z> ["release notes"]')
}
const notes = process.argv.slice(3).join(' ') || `NextAI Translator v${version}`

if (!fs.existsSync(KEY_PATH)) {
    fail(`signing key not found at ${KEY_PATH} — generate one with:\n  npx tauri signer generate -w src-tauri/keys/updater.key --ci`)
}

// ---------- repo slug from git remote ----------
let repoSlug = 'L1nze/nextai-translator'
try {
    const remote = execFileSync('git', ['remote', 'get-url', 'origin'], { encoding: 'utf8' }).trim()
    const m = remote.match(/github\.com[/:]([^/]+)\/([^/.]+)/)
    if (m) repoSlug = `${m[1]}/${m[2]}`
} catch {
    // keep default
}

// ---------- bump versions ----------
function bumpJson(file) {
    const p = JSON.parse(fs.readFileSync(file, 'utf8'))
    p.version = version
    fs.writeFileSync(file, JSON.stringify(p, null, 2) + '\n')
}
bumpJson(path.join(ROOT, 'package.json'))
bumpJson(path.join(ROOT, 'src-tauri', 'tauri.conf.json'))
console.log(`✔ version bumped to ${version}`)

// ---------- signed build ----------
console.log('▶ building signed NSIS bundle (renderer + Rust + bundler)...')
run('pnpm', ['build'], {
    cwd: ROOT,
    env: {
        ...process.env,
        // Path (native separators) + empty password so the bundler never
        // falls back to an interactive prompt and hangs the build.
        TAURI_SIGNING_PRIVATE_KEY: KEY_PATH,
        TAURI_SIGNING_PRIVATE_KEY_PASSWORD: '',
        // tauri.conf.json ships with createUpdaterArtifacts:false so daily
        // unsigned `pnpm build` runs do not fail at the signing step; the
        // release build needs the .sig updater artifact, so turn it back on.
        TAURI_CONFIG: JSON.stringify({ bundle: { createUpdaterArtifacts: true } }),
    },
})

// ---------- collect artifacts ----------
const bundleDir = path.join(ROOT, 'src-tauri', 'target', 'release', 'bundle', 'nsis')
const installerSrc = path.join(bundleDir, `NextAI Translator_${version}_x64-setup.exe`)
const sigSrc = installerSrc + '.sig'
if (!fs.existsSync(installerSrc)) fail(`installer not found: ${installerSrc}`)
if (!fs.existsSync(sigSrc)) fail(`signature not found: ${sigSrc} (was TAURI_SIGNING_PRIVATE_KEY set?)`)

const stageDir = path.join(ROOT, 'release', `v${version}`)
fs.mkdirSync(stageDir, { recursive: true })
// Dots instead of spaces so the download URL in latest.json stays clean.
const installerName = `NextAI.Translator_${version}_x64-setup.exe`
fs.copyFileSync(installerSrc, path.join(stageDir, installerName))
fs.copyFileSync(sigSrc, path.join(stageDir, installerName + '.sig'))

const signature = fs.readFileSync(sigSrc, 'utf8').trim()
const latest = {
    version,
    notes,
    pub_date: new Date().toISOString(),
    platforms: {
        'windows-x86_64': {
            signature,
            url: `https://github.com/${repoSlug}/releases/download/v${version}/${installerName}`,
        },
    },
}
fs.writeFileSync(path.join(stageDir, 'latest.json'), JSON.stringify(latest, null, 2) + '\n')

console.log(`✔ staged release artifacts in release/v${version}/`)
console.log(`
Publish steps (GitHub web UI, no gh CLI required):
  1. Push the version commits and create tag:  git tag v${version} && git push origin main --tags
  2. Open https://github.com/${repoSlug}/releases/new  → choose existing tag v${version}
  3. Title: v${version}   Description: ${notes}
  4. Attach BOTH files from release/v${version}/ :
       - ${installerName}
       - latest.json
  5. Publish release (must NOT be draft/prerelease — the updater reads
     /releases/latest/download/latest.json)

Installed apps check this endpoint every 10 minutes and will pick it up
automatically. Keep src-tauri/keys/updater.key safe — losing it breaks
all future updates.
`)
