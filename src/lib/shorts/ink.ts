import type { BlotShape, ShortStamp, ShortStyle } from '@/lib/api/types'
import { BLOT, FRAME } from '@/lib/shorts/frame'
import { rng, type Rng } from '@/lib/shorts/rng'

/**
 * What a short's still layers are drawn from, as SVG: the blots, the ground
 * under them, the stamp. Text in, text out - nothing here touches the page,
 * so the same seed gives the same ink wherever it is asked for.
 */

const f = (n: number) => n.toFixed(1)

/** How see-through a blot is over the ground. */
const BLOT_OPACITY = 0.3

/**
 * One ink blot on its own 900x900 canvas, its heart at (450, 400). The body,
 * the rays and the runs are plain shapes fused by a blur-and-threshold
 * filter, so the necks thin out and the joints round off the way ink does.
 */
function blotSvg(r: Rng, shape: BlotShape, color: string): string {
  const cx = BLOT.heartX
  const cy = BLOT.heartY
  const R = shape === 'spray' ? 78 : shape === 'smear' ? 100 : 112
  let goo = ''
  let loose = ''

  // The body: a circle bent by a few slow harmonics.
  const harmonics = [2, 3, 4, 5, 7].map(
    (k) => [k, (0.34 / k) * (0.4 + r()), r() * Math.PI * 2] as const,
  )
  const stretch = shape === 'smear' ? 1.75 : 1 + r() * 0.2
  const turn = r() * Math.PI
  let d = ''
  for (let i = 0; i < 96; i++) {
    const a = (i / 96) * Math.PI * 2
    const radius =
      R * (1 + harmonics.reduce((sum, [k, amp, ph]) => sum + amp * Math.sin(k * a + ph), 0))
    const px = Math.cos(a) * radius * stretch
    const py = (Math.sin(a) * radius) / Math.sqrt(stretch)
    d += `${i ? 'L' : 'M'}${f(cx + px * Math.cos(turn) - py * Math.sin(turn))} ${f(cy + px * Math.sin(turn) + py * Math.cos(turn))}`
  }
  goo += `<path d="${d}Z"/>`

  // Rays thrown out of the body, each ending in a bead; some throw a loose
  // droplet further on.
  const rays = {
    splat: 10 + Math.floor(r() * 5),
    drip: 4 + Math.floor(r() * 3),
    spray: 7 + Math.floor(r() * 4),
    smear: 6 + Math.floor(r() * 3),
  }[shape]
  for (let i = 0; i < rays; i++) {
    const a = ((i + r() * 0.8) / rays) * Math.PI * 2
    const len = R * (shape === 'splat' ? 1.5 + r() * 1.25 : 1.3 + r() * 0.9)
    const w = R * (0.1 + r() * 0.13)
    const ux = Math.cos(a)
    const uy = Math.sin(a)
    const from = R * 0.55
    const p = (dist: number, side: number) =>
      `${f(cx + ux * dist - uy * side)} ${f(cy + uy * dist + ux * side)}`
    goo += `<path d="M${p(from, w)}L${p(len, w * 0.22)}L${p(len, -w * 0.22)}L${p(from, -w)}Z"/>`
    const bead = w * (0.42 + r() * 0.45)
    goo += `<circle cx="${f(cx + ux * len)}" cy="${f(cy + uy * len)}" r="${f(bead)}"/>`
    if (r() < 0.6) {
      const far = len + bead * 2 + R * (0.12 + r() * 0.5)
      const x = cx + ux * far
      const y = cy + uy * far
      loose += `<ellipse cx="${f(x)}" cy="${f(y)}" rx="${f(bead * (0.7 + r() * 0.9))}" ry="${f(bead * 0.5)}" transform="rotate(${f((a * 180) / Math.PI)} ${f(x)} ${f(y)})"/>`
    }
  }

  // Runs: the ink that slid down the wall.
  const runs = shape === 'drip' ? 3 + Math.floor(r() * 3) : r() < 0.5 ? 1 : 0
  for (let i = 0; i < runs; i++) {
    const x = cx + (r() - 0.5) * R * 1.5
    const len = R * (1.0 + r() * 2.2)
    const w = R * (0.07 + r() * 0.08)
    goo += `<path d="M${f(x - w)} ${String(cy)}L${f(x - w * 0.55)} ${f(cy + len)}L${f(x + w * 0.55)} ${f(cy + len)}L${f(x + w)} ${String(cy)}Z"/>`
    goo += `<ellipse cx="${f(x)}" cy="${f(cy + len)}" rx="${f(w * 1.05)}" ry="${f(w * 1.5)}"/>`
  }

  // Spatter: many near and small, few far; the fast ones are stretched along
  // their flight.
  const drops = shape === 'spray' ? 70 : 34 + Math.floor(r() * 16)
  for (let i = 0; i < drops; i++) {
    const a = r() * Math.PI * 2
    const dist = R * (1.15 + Math.pow(r(), 1.6) * (shape === 'spray' ? 3.2 : 2.3))
    const size = Math.max(1.6, R * 0.1 * Math.pow(r(), 2.2) * (1.6 - dist / (R * 4.6)))
    const x = cx + Math.cos(a) * dist
    const y = cy + Math.sin(a) * dist
    loose += `<ellipse cx="${f(x)}" cy="${f(y)}" rx="${f(size * (1 + r() * 1.3))}" ry="${f(size)}" transform="rotate(${f((a * 180) / Math.PI)} ${f(x)} ${f(y)})"/>`
  }

  const side = String(BLOT.side)
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${side} ${side}" width="${side}" height="${side}">` +
    `<defs><filter id="goo" x="-10%" y="-10%" width="120%" height="120%" color-interpolation-filters="sRGB">` +
    `<feGaussianBlur stdDeviation="6"/><feColorMatrix values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 22 -9"/></filter></defs>` +
    `<g fill="${color}" opacity="${String(BLOT_OPACITY)}"><g filter="url(#goo)">${goo}</g>${loose}</g></svg>`
  )
}

/** How a blot drifts, and how far it is thrown sideways where the sound hits. */
export interface Drift {
  ax: number
  ay: number
  px: number
  py: number
  phase: number
  kick: number
}

/** The drift of the next blot, drawn after the blot itself. */
function driftOf(r: Rng): Drift {
  return {
    ax: 12 + r() * 16,
    ay: 12 + r() * 16,
    px: 6 + r() * 6,
    py: 6 + r() * 6,
    phase: r() * 6.28,
    kick: (r() < 0.5 ? -1 : 1) * (22 + r() * 30),
  }
}

/** A blot of the look, drawn: its picture, where its corner stands in the
 *  frame, how large it is there and how it drifts. */
export interface DrawnBlot extends Drift {
  svg: string
  x: number
  y: number
  size: number
}

/** The blots of a look, in order, each from the one generator the seed starts. */
export function blotsOf(look: ShortStyle): DrawnBlot[] {
  if (look.ground.pattern !== 'blots') return []
  const r = rng(look.ground.seed)
  return look.ground.blots.map((blot) => {
    const svg = blotSvg(
      r,
      blot.shape ?? 'splat',
      blot.color === 'second' ? look.second : look.accent,
    )
    return {
      svg,
      x: blot.x - blot.size / 2,
      y: blot.y - (blot.size * BLOT.heartY) / BLOT.side,
      size: blot.size,
      ...driftOf(r),
    }
  })
}

/** The ground under everything: its colour and what its pattern scatters on it. */
export function groundSvg(look: ShortStyle): string {
  const { width, height } = FRAME
  // Apart from the blots' own generator, so adding a blot moves no dust.
  const r = rng((look.ground.seed + 7919) >>> 0)
  const either = () => (r() < 0.5 ? look.accent : look.second)
  let body = `<rect width="${String(width)}" height="${String(height)}" fill="${look.ground.color}"/>`
  switch (look.ground.pattern) {
    case 'blots':
      // A little dust in the look's colours.
      for (let i = 0; i < 40; i++) {
        body += `<circle cx="${f(r() * width)}" cy="${f(r() * height)}" r="${f(1.2 + r() * 2.6)}" fill="${either()}" opacity="0.2"/>`
      }
      break
    case 'dots':
      for (let i = 0; i < 260; i++) {
        body += `<circle cx="${f(r() * width)}" cy="${f(r() * height)}" r="${f(2 + r() * 5)}" fill="${either()}" opacity="${f(0.12 + r() * 0.2)}"/>`
      }
      break
    case 'grid':
      for (let x = 60; x < width; x += 60) {
        body += `<rect x="${String(x)}" y="0" width="2" height="${String(height)}" fill="${look.accent}" opacity="0.12"/>`
      }
      for (let y = 60; y < height; y += 60) {
        body += `<rect x="0" y="${String(y)}" width="${String(width)}" height="2" fill="${look.second}" opacity="0.12"/>`
      }
      break
    case 'stripes':
      for (let at = -height; at < width; at += 120) {
        body += `<path d="M${String(at)} ${String(height)}L${String(at + height)} 0L${String(at + height + 44)} 0L${String(at + 44)} ${String(height)}Z" fill="${at % 240 === 0 ? look.accent : look.second}" opacity="0.1"/>`
      }
      break
    case 'plain':
      break
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${String(width)} ${String(height)}" width="${String(width)}" height="${String(height)}">${body}</svg>`
}

/** A text as an SVG may hold it. */
function escaped(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

/**
 * The worn print of a rubber die, as a filter: ink is either on the paper or
 * missing. Fine pits everywhere, and dry streaks along the fibres of the
 * paper - more of both the more worn it is. None for a clean print.
 */
export function wearFilter(wear: number, seed: number): string {
  const worn = Math.min(1, Math.max(0, wear))
  if (worn === 0) return ''
  const pits = f(-12 * (0.74 - 0.2 * worn))
  const fibres = f(-14 * (0.78 - 0.16 * worn))
  return (
    `<filter id="wear" x="0" y="0" width="100%" height="100%" color-interpolation-filters="sRGB">` +
    `<feTurbulence type="fractalNoise" baseFrequency="0.55" numOctaves="2" seed="${String(seed)}" result="n1"/>` +
    `<feColorMatrix in="n1" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 12 ${pits}" result="pits"/>` +
    `<feTurbulence type="fractalNoise" baseFrequency="0.012 0.42" numOctaves="2" seed="${String(seed + 1)}" result="n2"/>` +
    `<feColorMatrix in="n2" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 14 ${fibres}" result="fibres"/>` +
    `<feComposite in="SourceGraphic" in2="pits" operator="out" result="pitted"/>` +
    `<feComposite in="pitted" in2="fibres" operator="out"/></filter>`
  )
}

/** A seed for a stamp's wear, from what is stamped: the same name wears the
 *  same way every time. */
export function wearSeed(text: string): number {
  let sum = 451
  for (const char of text) sum = (sum + (char.codePointAt(0) ?? 0)) % 9973
  return sum
}

/**
 * The channel's name as a stamp or a plate, lettered: a double frame with
 * the name inside it, or a solid plate the name is cut out of - a second,
 * smaller line under it held between two rules. `null` when there is no name
 * to letter.
 */
export function stampSvg(stamp: ShortStamp): { svg: string; width: number; height: number } | null {
  const name = stamp.text.trim()
  if (name === '') return null
  const sub = stamp.sub.trim()
  const width = FRAME.stamp.width
  const height = sub === '' ? 150 : 190
  const ink = stamp.color
  const inner = width - 84
  const letters = `font-family="'Arial Black', Impact, 'Arial', sans-serif" font-weight="900" text-anchor="middle"`
  // The name is set to the frame's width whatever its length: a short name
  // is spaced out, a long one drawn narrower.
  let words = `<text x="${String(width / 2)}" y="${sub === '' ? '104' : '94'}" font-size="84" textLength="${String(inner)}" lengthAdjust="spacingAndGlyphs" ${letters}>${escaped(name.toUpperCase())}</text>`
  if (sub !== '') {
    const span = Math.min(inner * 0.5, 40 + sub.length * 26)
    const rule = (inner - span) / 2 - 18
    words +=
      `<text x="${String(width / 2)}" y="154" font-size="40" textLength="${f(span)}" lengthAdjust="spacingAndGlyphs" ${letters}>${escaped(sub.toUpperCase())}</text>` +
      `<rect x="42" y="136" width="${f(rule)}" height="6"/>` +
      `<rect x="${f(width - 42 - rule)}" y="136" width="${f(rule)}" height="6"/>`
  }
  const filter = wearFilter(stamp.wear, wearSeed(name))
  const worn = filter === '' ? '' : ' filter="url(#wear)"'
  const body =
    stamp.shape === 'plate'
      ? `<defs>${filter}<mask id="cut"><rect width="${String(width)}" height="${String(height)}" fill="white"/><g fill="black">${words}</g></mask></defs>` +
        `<g${worn}><rect x="4" y="4" width="${String(width - 8)}" height="${String(height - 8)}" rx="10" fill="${ink}" mask="url(#cut)"/></g>`
      : `<defs>${filter}</defs><g${worn} fill="${ink}">` +
        `<rect x="6" y="6" width="${String(width - 12)}" height="${String(height - 12)}" rx="10" fill="none" stroke="${ink}" stroke-width="10"/>` +
        `<rect x="24" y="24" width="${String(width - 48)}" height="${String(height - 48)}" rx="5" fill="none" stroke="${ink}" stroke-width="3.5"/>` +
        `${words}</g>`
  return {
    svg: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${String(width)} ${String(height)}" width="${String(width)}" height="${String(height)}">${body}</svg>`,
    width,
    height,
  }
}

/** A picture of the channel's own, worn: the picture inside an SVG that puts
 *  the wear on it. `src` is an address a picture is read from. */
export function wornPicture(
  src: string,
  width: number,
  height: number,
  wear: number,
  seed: number,
): string {
  const filter = wearFilter(wear, seed)
  const worn = filter === '' ? '' : ' filter="url(#wear)"'
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${String(width)} ${String(height)}" width="${String(width)}" height="${String(height)}">` +
    `<defs>${filter}</defs><g${worn}><image href="${escaped(src)}" width="${String(width)}" height="${String(height)}"/></g></svg>`
  )
}

/** The address a picture kept as text is read from: the text of an SVG as a
 *  `data:` address, anything else as it is. Empty for none. */
export function artSrc(art: string): string {
  const text = art.trim()
  if (text === '') return ''
  if (text.startsWith('<')) return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(text)}`
  return text
}
