import type { JsonValue, ShortStyle } from '@/lib/api/types'
import { FRAME } from '@/lib/shorts/frame'
import {
  artSrc,
  blotsOf,
  groundSvg,
  stampSvg,
  wearSeed,
  wornPicture,
  type Drift,
} from '@/lib/shorts/ink'

/**
 * The still layers of a short, drawn in the window: the ground, the blots,
 * the header, the stamp and the sign, each on its own canvas.
 *
 * One code path for both uses - the frame previewed while the style is
 * edited, and the pictures handed to the builder - so what is seen is what
 * is built. The builder only moves them.
 */

/** A blot on its canvas, with where its corner stands and how it drifts. */
export interface BlotLayer extends Drift {
  canvas: HTMLCanvasElement
  x: number
  y: number
}

/** Everything of a look that stands still, drawn. */
export interface Drawn {
  ground: HTMLCanvasElement
  blots: BlotLayer[]
  /** The title and the call on a clear frame; none when the header is off. */
  header: HTMLCanvasElement | null
  /** The stamp, tilted, with its ghost in the second colour and where its
   *  corner stands. */
  stamp: { canvas: HTMLCanvasElement; ghost: HTMLCanvasElement; x: number; y: number } | null
  /** The sign at its largest on its square, and with its eyes shut. */
  sign: { open: HTMLCanvasElement; blink: HTMLCanvasElement | null } | null
}

function canvasOf(width: number, height: number): HTMLCanvasElement {
  const canvas = document.createElement('canvas')
  canvas.width = Math.max(1, Math.round(width))
  canvas.height = Math.max(1, Math.round(height))
  return canvas
}

function contextOf(canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const context = canvas.getContext('2d')
  if (context === null) throw new Error('no canvas to draw a short on')
  return context
}

/** A picture, read: from an address, or from the text of an SVG. */
export function pictureOf(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image()
    image.onload = () => resolve(image)
    image.onerror = () => reject(new Error('a picture of the short style does not read'))
    image.src = src
  })
}

const svgSrc = (svg: string) => `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`

/** An SVG drawn onto a canvas of the size given. */
async function drawnSvg(svg: string, width: number, height: number): Promise<HTMLCanvasElement> {
  const image = await pictureOf(svgSrc(svg))
  const canvas = canvasOf(width, height)
  contextOf(canvas).drawImage(image, 0, 0, canvas.width, canvas.height)
  return canvas
}

/** A picture fitted whole into a clear square. */
async function fitted(art: string, side: number): Promise<HTMLCanvasElement | null> {
  const src = artSrc(art)
  if (src === '') return null
  const image = await pictureOf(src)
  const natural = { w: image.naturalWidth || side, h: image.naturalHeight || side }
  const scale = Math.min(side / natural.w, side / natural.h)
  const canvas = canvasOf(side, side)
  const w = natural.w * scale
  const h = natural.h * scale
  contextOf(canvas).drawImage(image, (side - w) / 2, (side - h) / 2, w, h)
  return canvas
}

/** A canvas turned about its middle, on a canvas grown to hold it whole. */
function tilted(source: HTMLCanvasElement, degrees: number): HTMLCanvasElement {
  const angle = (degrees * Math.PI) / 180
  const sin = Math.abs(Math.sin(angle))
  const cos = Math.abs(Math.cos(angle))
  const canvas = canvasOf(
    source.width * cos + source.height * sin,
    source.width * sin + source.height * cos,
  )
  const context = contextOf(canvas)
  context.translate(canvas.width / 2, canvas.height / 2)
  context.rotate(angle)
  context.drawImage(source, -source.width / 2, -source.height / 2)
  return canvas
}

/** The same shape in one colour, a little see-through: the stamp's ghost. */
function ghostOf(source: HTMLCanvasElement, color: string): HTMLCanvasElement {
  const canvas = canvasOf(source.width, source.height)
  const context = contextOf(canvas)
  context.globalAlpha = 0.7
  context.drawImage(source, 0, 0)
  context.globalAlpha = 1
  context.globalCompositeOperation = 'source-in'
  context.fillStyle = color
  context.fillRect(0, 0, canvas.width, canvas.height)
  return canvas
}

/** The size a title is lettered at so it fits the frame. */
function fittedSize(
  context: CanvasRenderingContext2D,
  text: string,
  font: (size: number) => string,
): number {
  context.font = font(FRAME.title.size)
  const width = context.measureText(text).width
  return width <= FRAME.title.fit
    ? FRAME.title.size
    : Math.max(24, Math.floor((FRAME.title.size * FRAME.title.fit) / width))
}

/** The title and the call, lettered on a clear frame. */
function headerOf(look: ShortStyle, title: string): HTMLCanvasElement | null {
  if (!look.header.on) return null
  const canvas = canvasOf(FRAME.width, FRAME.height)
  const context = contextOf(canvas)
  const family = `${JSON.stringify(look.header.font.trim() || 'Arial')}, Arial, sans-serif`
  context.textAlign = 'center'
  context.textBaseline = 'top'
  context.lineJoin = 'round'
  const lettered = (text: string, top: number, font: string, fill: string, edge: number) => {
    context.font = font
    context.lineWidth = edge * 2
    // The edge every word on a frame is read by, over any ground.
    context.strokeStyle = 'rgba(0, 0, 0, 0.75)'
    context.strokeText(text, FRAME.width / 2, top)
    context.fillStyle = fill
    context.fillText(text, FRAME.width / 2, top)
  }
  const shown = look.header.title_upper ? title.toUpperCase() : title
  if (shown.trim() !== '') {
    const size = fittedSize(context, shown, (px) => `bold ${String(px)}px ${family}`)
    lettered(shown, FRAME.title.top, `bold ${String(size)}px ${family}`, 'white', 3)
  }
  const call = look.header.call.trim()
  if (call !== '') {
    lettered(call, FRAME.call.top, `${String(FRAME.call.size)}px ${family}`, look.second, 3)
  }
  return canvas
}

/** The stamp of a look: the channel's own picture, or the name lettered -
 *  worn, tilted, with its ghost. */
async function stampOf(look: ShortStyle): Promise<Drawn['stamp']> {
  const stamp = look.stamp
  if (!stamp.on) return null
  let flat: HTMLCanvasElement
  const own = artSrc(stamp.art)
  if (own !== '') {
    const image = await pictureOf(own)
    const natural = { w: image.naturalWidth || FRAME.stamp.width, h: image.naturalHeight || 180 }
    const height = (FRAME.stamp.width * natural.h) / natural.w
    flat = await drawnSvg(
      wornPicture(own, natural.w, natural.h, stamp.wear, wearSeed(stamp.text || 'stamp')),
      FRAME.stamp.width,
      height,
    )
  } else {
    const lettered = stampSvg(stamp)
    if (lettered === null) return null
    flat = await drawnSvg(lettered.svg, lettered.width, lettered.height)
  }
  const canvas = tilted(flat, stamp.angle)
  return {
    canvas,
    ghost: ghostOf(canvas, look.second),
    x: Math.round(stamp.x - canvas.width / 2),
    y: Math.round(FRAME.stamp.middle - canvas.height / 2),
  }
}

/** Draw everything of `look` that stands still, lettering `title`. */
export async function draw(look: ShortStyle, title: string): Promise<Drawn> {
  const ground = await drawnSvg(groundSvg(look), FRAME.width, FRAME.height)
  const blots: BlotLayer[] = []
  for (const { svg, x, y, size, ...drift } of blotsOf(look)) {
    blots.push({ canvas: await drawnSvg(svg, size, size), x, y, ...drift })
  }
  let sign: Drawn['sign'] = null
  if (look.sign.on) {
    const open = await fitted(look.sign.art, FRAME.sign.box)
    if (open !== null) sign = { open, blink: await fitted(look.sign.blink, FRAME.sign.box) }
  }
  return { ground, blots, header: headerOf(look, title), stamp: await stampOf(look), sign }
}

/** What a preview shows in place of what only a build has. */
export interface Sample {
  /** The picture that stands for the video: the cover, when there is one. */
  picture: HTMLImageElement | null
  /** A line of words, the first of them sung. */
  words: string
}

/** A wave as a preview draws it: the same shape every time, so a frame does
 *  not twitch while a number is typed. */
function sampleLevel(at: number): number {
  return (
    0.18 + 0.82 * Math.abs(Math.sin(at * 12.9898) * Math.sin(at * 3.7 + 1.3) * Math.cos(at * 0.71))
  )
}

/** One frame of the short, a third of the way in, on a 1080x1920 canvas. */
export function paint(
  canvas: HTMLCanvasElement,
  look: ShortStyle,
  drawn: Drawn,
  sample: Sample,
): void {
  canvas.width = FRAME.width
  canvas.height = FRAME.height
  const context = contextOf(canvas)
  context.drawImage(drawn.ground, 0, 0)
  for (const blot of drawn.blots) context.drawImage(blot.canvas, blot.x, blot.y)

  // The picture: the cover cut to 16:9, or a dark plate where it will be.
  const { top, height } = FRAME.picture
  context.fillStyle = 'black'
  context.fillRect(0, top, FRAME.width, height)
  if (sample.picture !== null && sample.picture.naturalWidth > 0) {
    const natural = { w: sample.picture.naturalWidth, h: sample.picture.naturalHeight }
    const scale = Math.max(FRAME.width / natural.w, height / natural.h)
    const w = FRAME.width / scale
    const h = height / scale
    context.drawImage(
      sample.picture,
      (natural.w - w) / 2,
      (natural.h - h) / 2,
      w,
      h,
      0,
      top,
      FRAME.width,
      height,
    )
  }
  if (look.effects.vignette) {
    const shade = context.createRadialGradient(
      FRAME.width / 2,
      top + height / 2,
      height * 0.35,
      FRAME.width / 2,
      top + height / 2,
      FRAME.width * 0.62,
    )
    shade.addColorStop(0, 'rgba(0, 0, 0, 0)')
    shade.addColorStop(1, 'rgba(0, 0, 0, 0.55)')
    context.fillStyle = shade
    context.fillRect(0, top, FRAME.width, height)
  }

  if (look.wave.on) {
    if (look.wave.strip) {
      context.globalAlpha = 0.22
      context.fillStyle = look.accent
      context.fillRect(0, FRAME.strip.top, FRAME.width, FRAME.strip.height)
      context.globalAlpha = 1
      context.fillRect(0, FRAME.strip.top, FRAME.width / 3, FRAME.strip.height)
    }
    const middle = FRAME.wave.top + FRAME.wave.height / 2
    const half = FRAME.wave.height / 2
    if (look.wave.kind === 'bars') {
      context.fillStyle = look.accent
      for (let k = 0; k < 28; k++) {
        const tall = 4 + sampleLevel(k * 0.37 + 1) * (1 - k / 40) * (half - 8)
        for (const side of [-1, 1]) {
          context.fillRect(FRAME.width / 2 + side * (k * 19 + 9.5) - 6, middle - tall, 12, tall * 2)
        }
      }
    } else if (look.wave.kind === 'lines') {
      for (const [color, shift] of [
        [look.accent, 0],
        [look.second, 0.9],
      ] as const) {
        context.strokeStyle = color
        context.lineWidth = 3
        context.beginPath()
        for (let x = 0; x <= FRAME.width; x += 6) {
          const y =
            middle + Math.sin(x * 0.045 + shift * 4) * sampleLevel(x * 0.011 + shift) * half * 0.8
          if (x === 0) context.moveTo(x, y)
          else context.lineTo(x, y)
        }
        context.stroke()
      }
    } else {
      context.fillStyle = look.accent
      for (let x = 0; x < FRAME.width; x += 3) {
        const tall = sampleLevel(x * 0.017) * half * 0.86
        context.fillRect(x, middle - tall, 3, tall * 2)
      }
    }
  }

  if (drawn.header !== null) context.drawImage(drawn.header, 0, 0)

  if (look.karaoke.on && sample.words.trim() !== '') {
    const words = sample.words.trim().split(/\s+/)
    const size = look.karaoke.size
    context.font = `bold ${String(size)}px ${JSON.stringify(look.karaoke.font.trim() || 'Arial')}, Arial, sans-serif`
    context.textBaseline = 'top'
    context.textAlign = 'left'
    context.lineJoin = 'round'
    const gap = context.measureText(' ').width
    const widths = words.map((word) => context.measureText(word).width)
    let x = (FRAME.width - widths.reduce((sum, w) => sum + w, 0) - gap * (words.length - 1)) / 2
    words.forEach((word, at) => {
      context.lineWidth = 8
      context.strokeStyle = 'rgba(16, 16, 16, 1)'
      context.strokeText(word, x, FRAME.words.top)
      // The first two words are sung already.
      context.fillStyle = at < 2 ? look.accent : 'white'
      context.fillText(word, x, FRAME.words.top)
      x += (widths[at] ?? 0) + gap
    })
  }

  if (drawn.stamp !== null) context.drawImage(drawn.stamp.canvas, drawn.stamp.x, drawn.stamp.y)

  if (drawn.sign !== null) {
    const side = Math.min(FRAME.sign.box, look.sign.size * (1 + look.sign.pulse * 0.4))
    context.drawImage(
      drawn.sign.open,
      (FRAME.width - side) / 2,
      FRAME.sign.top + (FRAME.sign.box - side) / 2,
      side,
      side,
    )
  }
}

/** A canvas as the bytes of a PNG. */
async function png(canvas: HTMLCanvasElement): Promise<number[]> {
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'))
  if (blob === null) throw new Error('a layer of the short could not be written')
  return Array.from(new Uint8Array(await blob.arrayBuffer()))
}

/** The layers as files, and what each is and where it stands - as the
 *  builder is handed them. */
export async function layersOf(
  drawn: Drawn,
): Promise<{ placed: JsonValue; layers: { name: string; bytes: number[] }[] }> {
  const layers: { name: string; bytes: number[] }[] = []
  const laid = async (name: string, canvas: HTMLCanvasElement) => {
    layers.push({ name, bytes: await png(canvas) })
    return name
  }
  const ground = await laid('ground.png', drawn.ground)
  const blots = []
  for (const [at, { canvas, ...place }] of drawn.blots.entries()) {
    blots.push({ file: await laid(`blot-${String(at)}.png`, canvas), ...place })
  }
  const header = drawn.header === null ? null : await laid('header.png', drawn.header)
  const stamp =
    drawn.stamp === null
      ? null
      : {
          file: await laid('stamp.png', drawn.stamp.canvas),
          ghost: await laid('stamp-ghost.png', drawn.stamp.ghost),
          x: drawn.stamp.x,
          y: drawn.stamp.y,
        }
  const sign =
    drawn.sign === null
      ? null
      : {
          open: await laid('sign.png', drawn.sign.open),
          blink: drawn.sign.blink === null ? null : await laid('sign-blink.png', drawn.sign.blink),
        }
  return { placed: { ground, header, blots, stamp, sign }, layers }
}
