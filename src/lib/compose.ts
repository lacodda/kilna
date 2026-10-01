/*
 * A cover with the channel's mark laid over it, composed in the window.
 *
 * A generator distorts a logo, so the mark is put on the finished picture
 * as its own file (v0.88): the window draws both on a canvas and hands the
 * bytes back to be saved. Both files arrive as bytes rather than as URLs of
 * the asset protocol - a picture fetched from another origin taints the
 * canvas and its bytes could not be read back out.
 */

/**
 * Raw bytes from the backend as an `ArrayBuffer`, whichever shape the IPC
 * channel delivered them in: a buffer over the IPC protocol, an array of
 * numbers over the message channel.
 */
export function bytesOf(raw: ArrayBuffer | readonly number[]): ArrayBuffer {
  return raw instanceof ArrayBuffer ? raw : Uint8Array.from(raw).buffer
}

/** A box as shares of a picture's width and height. */
export interface Box {
  x: number
  y: number
  w: number
  h: number
}

const TYPES: Record<string, string> = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  webp: 'image/webp',
  gif: 'image/gif',
  avif: 'image/avif',
  svg: 'image/svg+xml',
}

/** The type a file's ending says it is, PNG when it says nothing known. */
export function typeOf(name: string): string {
  const ending = name.split('.').pop()?.toLowerCase() ?? ''
  return TYPES[ending] ?? 'image/png'
}

/**
 * Where the mark is drawn in a picture `width` by `height`: the largest
 * square inside the box, pushed into the box's corner of the picture - the
 * box is square in the cover's shape, and a picture of a slightly other
 * shape would stretch the mark.
 */
export function markPlacement(
  box: Box,
  width: number,
  height: number,
): { x: number; y: number; side: number } {
  const left = box.x * width
  const top = box.y * height
  const wide = box.w * width
  const tall = box.h * height
  const side = Math.min(wide, tall)
  const right = box.x + box.w / 2 > 0.5
  const low = box.y + box.h / 2 > 0.5
  return {
    x: right ? left + wide - side : left,
    y: low ? top + tall - side : top,
    side,
  }
}

async function load(bytes: ArrayBuffer, name: string): Promise<HTMLImageElement> {
  const url = URL.createObjectURL(new Blob([bytes], { type: typeOf(name) }))
  try {
    return await new Promise<HTMLImageElement>((resolve, reject) => {
      const image = new Image()
      image.onload = () => resolve(image)
      image.onerror = () => reject(new Error(`could not read ${name} as a picture`))
      image.src = url
    })
  } finally {
    URL.revokeObjectURL(url)
  }
}

/** The picture with the mark drawn over it in `box`, as PNG bytes. */
export async function composeWithMark(
  picture: { bytes: ArrayBuffer; name: string },
  mark: { bytes: ArrayBuffer; name: string },
  box: Box,
): Promise<Uint8Array> {
  const [base, sign] = await Promise.all([
    load(picture.bytes, picture.name),
    load(mark.bytes, mark.name),
  ])
  const canvas = document.createElement('canvas')
  canvas.width = base.naturalWidth
  canvas.height = base.naturalHeight
  const context = canvas.getContext('2d')
  if (context === null) throw new Error('the window cannot draw a picture')
  context.drawImage(base, 0, 0)
  const at = markPlacement(box, canvas.width, canvas.height)
  // The mark keeps its own proportions inside the square.
  const ratio = (sign.naturalWidth || 1) / (sign.naturalHeight || 1)
  const drawnWide = ratio >= 1 ? at.side : at.side * ratio
  const drawnTall = ratio >= 1 ? at.side / ratio : at.side
  context.drawImage(
    sign,
    at.x + (at.side - drawnWide) / 2,
    at.y + (at.side - drawnTall) / 2,
    drawnWide,
    drawnTall,
  )
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'))
  if (blob === null) throw new Error('the window could not write the picture')
  return new Uint8Array(await blob.arrayBuffer())
}
