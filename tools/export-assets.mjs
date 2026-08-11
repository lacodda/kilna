// Rasterizes the kilna brand SVGs into PNGs, a multi-size .ico, and the icon
// set Tauri bundles. Masters live in the vault brand registry; SVGs here are
// copies of them and are never edited by hand.
//
//   pnpm exec node tools/export-assets.mjs
import sharp from 'sharp'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const ASSETS = path.join(ROOT, 'assets')
const TAURI_ICONS = path.join(ROOT, 'src-tauri', 'icons')

// The S tile (filled hex, bold code) is what reads at icon sizes.
const S = path.join(ASSETS, 'logo-s.svg')
const L = path.join(ASSETS, 'logo.svg')
const BANNER = path.join(ASSETS, 'banner.svg')

const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]
// Sizes Tauri's bundler expects on Windows, macOS and Linux.
const TAURI_PNG = [32, 128, 256, 512]

async function png(src, size, out) {
  await sharp(src, { density: 384 }).resize(size, size).png().toFile(out)
}

// Minimal ICO container: header + directory entries + embedded PNG payloads.
// sharp cannot write this container itself.
function buildIco(pngBuffers, sizes) {
  const count = pngBuffers.length
  const header = Buffer.alloc(6)
  header.writeUInt16LE(0, 0) // reserved
  header.writeUInt16LE(1, 2) // type: icon
  header.writeUInt16LE(count, 4)

  const entries = Buffer.alloc(16 * count)
  let offset = 6 + 16 * count
  pngBuffers.forEach((buf, i) => {
    const size = sizes[i]
    const e = 16 * i
    entries.writeUInt8(size >= 256 ? 0 : size, e + 0) // width (0 means 256)
    entries.writeUInt8(size >= 256 ? 0 : size, e + 1) // height
    entries.writeUInt8(0, e + 2) // palette
    entries.writeUInt8(0, e + 3) // reserved
    entries.writeUInt16LE(1, e + 4) // colour planes
    entries.writeUInt16LE(32, e + 6) // bits per pixel
    entries.writeUInt32LE(buf.length, e + 8)
    entries.writeUInt32LE(offset, e + 12)
    offset += buf.length
  })

  return Buffer.concat([header, entries, ...pngBuffers])
}

fs.mkdirSync(TAURI_ICONS, { recursive: true })

const icoParts = []
for (const size of ICO_SIZES) {
  icoParts.push(await sharp(S, { density: 384 }).resize(size, size).png().toBuffer())
}
const ico = buildIco(icoParts, ICO_SIZES)
fs.writeFileSync(path.join(ASSETS, 'icon.ico'), ico)
fs.writeFileSync(path.join(TAURI_ICONS, 'icon.ico'), ico)
console.log('wrote icon.ico')

await png(S, 32, path.join(ASSETS, 'favicon-32.png'))
await png(S, 180, path.join(ASSETS, 'apple-touch-icon.png'))
await png(L, 512, path.join(ASSETS, 'logo-512.png'))

// Tauri resolves these by name from tauri.conf.json's bundle.icon list.
for (const size of TAURI_PNG) {
  const name = size === 512 ? 'icon.png' : `${size}x${size}.png`
  await png(S, size, path.join(TAURI_ICONS, name))
}
console.log('wrote pngs')

// GitHub social preview: 1280x640 on the line's tile colour. The banner plate
// spans the full 720px while the artwork fills the left ~570px, so the tail is
// trimmed; the rounded plate over an identical background would leave a seam.
const bannerWidth = 1600
const bannerHeight = Math.round((bannerWidth * 170) / 720)
const inset = Math.round((bannerWidth * 6) / 720) // clears the plate's rounded edge
const banner = await sharp(BANNER, { density: 384 })
  .resize({ width: bannerWidth })
  .extract({ left: inset, top: inset, width: 1290 - inset, height: bannerHeight - 2 * inset })
  .png()
  .toBuffer()

await sharp({
  create: { width: 1280, height: 640, channels: 4, background: '#1B2126' },
})
  .composite([{ input: await sharp(banner).resize({ width: 880 }).png().toBuffer(), gravity: 'centre' }])
  .png()
  .toFile(path.join(ASSETS, 'social-preview.png'))
console.log('wrote social-preview.png')
