#!/usr/bin/env node
// Keeps the snapshot of the style registry that ships with kilna honest
// (v0.137): every pack in `index.json` gets the SHA-256 of its manifest's
// bytes, the number of its styles and the day it last changed. kilna refuses
// a pack whose file does not match its digest, so a pack edited by hand is
// a pack nobody can install until this runs.
//
//   node tools/style-registry.mjs          rewrite the index
//   node tools/style-registry.mjs --check  say what is out of date, exit 1

import { createHash } from 'node:crypto'
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const root = join(import.meta.dirname, '..', 'src-tauri', 'resources', 'style-registry')
const indexPath = join(root, 'index.json')
const index = JSON.parse(readFileSync(indexPath, 'utf8'))
const check = process.argv.includes('--check')
const today = new Date().toISOString().slice(0, 10)

let stale = 0
for (const entry of index.packs) {
  const bytes = readFileSync(join(root, entry.manifest_url))
  const digest = createHash('sha256').update(bytes).digest('hex')
  const manifest = JSON.parse(bytes.toString('utf8'))
  const count = manifest.bricks.length
  if (manifest.id !== entry.id) {
    console.error(`${entry.id}: its file says it is ${manifest.id}`)
    process.exitCode = 1
  }
  if (entry.digest !== digest || entry.brick_count !== count) {
    stale += 1
    if (check) {
      console.error(`${entry.id}: the index is out of date`)
    } else {
      entry.digest = digest
      entry.brick_count = count
      entry.updated_at = today
    }
  }
}

if (check) {
  if (stale > 0) process.exitCode = 1
} else {
  writeFileSync(indexPath, `${JSON.stringify(index, null, 2)}\n`)
  console.log(`${index.packs.length} packs, ${stale} brought up to date`)
}
