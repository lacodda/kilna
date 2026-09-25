// Holds every registry copy in `src/components/ui/` to the registry it came from.
//
// A lowercase file there is a copy of a dowel component and is never edited;
// a PascalCase one is this app's own. The convention was only a convention:
// by 24.09 twelve copies had drifted - seven patched for a popup layer of
// kilna's own, one with a hand-set height, the rest simply left behind an
// upgrade - and `dowel diff` said so to anyone who ran it, which nobody did.
// A copy that differs is either a fix dowel should have, or a copy of an older
// dowel than the one installed; neither should be found by accident.
//
// The registry is read from the installed package, so upgrading dowel-ui and
// not re-copying fails here. `pnpm exec dowel diff <name>` shows the lines.
//
//   node tools/check-registry.mjs
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('../', import.meta.url))
const UI = join(ROOT, 'src/components/ui')
const REGISTRY = join(ROOT, 'node_modules/dowel-ui/dist/registry.json')

/** Copies allowed to differ, each with the reason. Empty is the goal; an entry
 * here is a debt with a name, not a way to make the check pass. */
const EXCEPTIONS = new Map()

const registry = JSON.parse(readFileSync(REGISTRY, 'utf8'))
const { version } = JSON.parse(
  readFileSync(join(ROOT, 'node_modules/dowel-ui/package.json'), 'utf8'),
)
const upstream = new Map()
for (const item of registry.items) {
  for (const file of item.files ?? []) {
    const name = file.path.replace(/^.*\//, '')
    upstream.set(name, file.content)
  }
}

/** Line endings are the checkout's business, not the copy's. */
const normal = (text) => text.replace(/\r\n/g, '\n')

const drifted = []
const strays = []
let copies = 0

for (const name of readdirSync(UI).sort()) {
  if (!/\.tsx?$/.test(name) || /\.test\.tsx?$/.test(name)) continue
  // PascalCase is kilna's own.
  if (!/^[a-z]/.test(name)) continue
  if (EXCEPTIONS.has(name)) continue
  const theirs = upstream.get(name)
  if (theirs === undefined) {
    strays.push(name)
    continue
  }
  copies += 1
  if (normal(readFileSync(join(UI, name), 'utf8')) !== normal(theirs)) drifted.push(name)
}

if (drifted.length > 0 || strays.length > 0) {
  for (const name of drifted) {
    console.error(
      `${name}: differs from dowel-ui ${version}` +
        ` - take the registry's copy (pnpm exec dowel diff ${name.replace(/\.tsx?$/, '')})`,
    )
  }
  for (const name of strays) {
    console.error(
      `${name}: a lowercase file with no registry twin - a copy dowel no longer ships, or kilna's own file named like a copy (PascalCase)`,
    )
  }
  process.exit(1)
}

console.log(`registry: ${copies} copies match dowel-ui ${version}`)
