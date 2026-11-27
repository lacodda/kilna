/** Whether a text is a colour as a profile keeps one: `#RRGGBB`. */
export function isColour(text: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(text)
}

/** What the system's colour picker is shown while the text is not a colour
 *  yet: it takes nothing else. */
export function pickerValue(text: string): string {
  return isColour(text) ? text : '#000000'
}
