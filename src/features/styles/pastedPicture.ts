/*
 * A picture pasted with Ctrl+V, the cheapest way a reference arrives: a
 * picture looked at in a generator's browser tab is copied from there, and
 * the paste hands over its bytes with no trip through the disk.
 */

/** A pasted picture the way the backend takes one. */
export interface PastedPicture {
  bytes: number[]
  /** What to remember it by. */
  name: string
}

/**
 * The picture a paste carries, if it carries one. Only a picture: text pasted
 * into the name or the description is the box's business, and taking it here
 * would break typing.
 */
export function pictureIn(event: ClipboardEvent): File | undefined {
  return Array.from(event.clipboardData?.files ?? []).find((file) => file.type.startsWith('image/'))
}

/** Its bytes, and a name - a picture copied from a browser tab often has
 * none, and a file without one is hard to find again. */
export async function readPicture(file: File): Promise<PastedPicture> {
  const extension = file.type.split('/')[1] ?? 'png'
  const buffer = await file.arrayBuffer()
  return {
    bytes: Array.from(new Uint8Array(buffer)),
    name: file.name === '' ? `pasted-${String(Date.now())}.${extension}` : file.name,
  }
}
