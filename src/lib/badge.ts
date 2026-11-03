import { Image } from '@tauri-apps/api/image'
import type { TFunction } from 'i18next'
import type { Activity } from '@/lib/api/types'

/**
 * The number on kilna's icon while work is going (v0.107): on the taskbar
 * while the window is open, on the tray while it is closed.
 *
 * Drawn here, in the page, because the page has the theme's accent and a
 * canvas to draw a digit with; the system only takes a picture. Windows has
 * no badge count of its own - it lays a small picture over the taskbar
 * button - so the same picture serves both places.
 */

/** What the badge says: a digit, or "9+" past what a disc can hold. */
export function badgeText(count: number): string {
  return count > 9 ? '9+' : String(count)
}

/** The tray's tooltip: what is going and what waits, or that all is done. */
export function trayLine(activity: Activity, t: TFunction): string {
  const { running, queued } = activity
  if (running === 0 && queued === 0) return t('outside.trayIdle')
  const going = t('outside.trayRunning', { count: running })
  return queued > 0 ? `${going}, ${t('outside.trayQueued', { count: queued })}` : going
}

/** The theme's accent and the ink that reads on it, as the page has them. */
function colours(): { fill: string; ink: string } {
  const style = getComputedStyle(document.documentElement)
  const read = (name: string, fallback: string) => style.getPropertyValue(name).trim() || fallback
  return { fill: read('--accent', 'magenta'), ink: read('--on-accent', 'white') }
}

/** A canvas of `size` square, or null where there is none to draw on. */
function canvas(size: number): CanvasRenderingContext2D | null {
  const element = document.createElement('canvas')
  element.width = size
  element.height = size
  try {
    return element.getContext('2d')
  } catch {
    // jsdom answers "not implemented" with a throw rather than a null.
    return null
  }
}

/** A disc with the count in it, centred in `size` - the taskbar's overlay. */
export async function badgeDisc(count: number, size = 32): Promise<Image | null> {
  const context = canvas(size)
  if (context === null) return null
  disc(context, count, size / 2, size / 2, size / 2)
  return picture(context, size)
}

/**
 * kilna's icon with the count in a disc at its lower right - the tray's -
 * or the icon alone when nothing is going.
 */
export async function badgedIcon(base: Image, count: number, size = 32): Promise<Image | null> {
  const context = canvas(size)
  if (context === null) return null
  const { width, height } = await base.size()
  const source = canvas(Math.max(width, height))
  if (source === null) return null
  source.putImageData(new ImageData(new Uint8ClampedArray(await base.rgba()), width, height), 0, 0)
  context.imageSmoothingQuality = 'high'
  context.drawImage(source.canvas, 0, 0, width, height, 0, 0, size, size)
  if (count > 0) {
    const radius = size * 0.3
    disc(context, count, size - radius, size - radius, radius)
  }
  return picture(context, size)
}

function disc(
  context: CanvasRenderingContext2D,
  count: number,
  x: number,
  y: number,
  radius: number,
) {
  const { fill, ink } = colours()
  context.fillStyle = fill
  context.beginPath()
  context.arc(x, y, radius, 0, 2 * Math.PI)
  context.fill()
  const text = badgeText(count)
  context.fillStyle = ink
  // Two characters need a smaller face to stay inside the disc.
  context.font = `700 ${Math.round(radius * (text.length > 1 ? 1.05 : 1.4))}px system-ui, sans-serif`
  context.textAlign = 'center'
  context.textBaseline = 'middle'
  context.fillText(text, x, y + radius * 0.08)
}

function picture(context: CanvasRenderingContext2D, size: number): Promise<Image> {
  const { data } = context.getImageData(0, 0, size, size)
  return Image.new(new Uint8Array(data.buffer, data.byteOffset, data.byteLength), size, size)
}
