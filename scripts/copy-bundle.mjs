#!/usr/bin/env node
// Post-build step: copy the freshly produced NSIS installer into release/
// so everyday packaging (`pnpm build`) lands there by default.
//
// Only files produced within the last 10 minutes are copied — after a failed
// build the bundle directory still holds the previous installer, and copying
// that would silently ship a stale artifact.

import fs from 'node:fs'
import path from 'node:path'

const ROOT = path.resolve(import.meta.dirname, '..')
const bundleDir = path.join(ROOT, 'src-tauri', 'target', 'release', 'bundle', 'nsis')
const releaseDir = path.join(ROOT, 'release')

function fail(msg) {
    console.error(`✖ ${msg}`)
    process.exit(1)
}

if (!fs.existsSync(bundleDir)) {
    fail(`bundle directory not found: ${bundleDir}`)
}

const candidates = fs
    .readdirSync(bundleDir)
    .filter((f) => f.endsWith('-setup.exe'))
    .map((f) => ({ f, mtime: fs.statSync(path.join(bundleDir, f)).mtimeMs }))
    .sort((a, b) => b.mtime - a.mtime)

if (candidates.length === 0) {
    fail('no NSIS installer found in bundle/nsis — the build did not produce one')
}

const newest = candidates[0]
if (Date.now() - newest.mtime > 10 * 60 * 1000) {
    fail(`newest installer is ${Math.round((Date.now() - newest.mtime) / 60000)}min old — refusing to copy a stale artifact`)
}

fs.mkdirSync(releaseDir, { recursive: true })
fs.copyFileSync(path.join(bundleDir, newest.f), path.join(releaseDir, newest.f))
console.log(`✔ copied ${newest.f} -> release/`)
