import type { ShortStyle, ShortVariant } from '@/lib/api/types'

/**
 * The style as a variant wears it: what the variant names replaces the
 * style's, what it leaves out is the style's. The same rule the backend
 * builds by (`ShortStyle::worn`), here for the constructor's preview of a
 * draft that is not saved yet.
 */
export function worn(style: ShortStyle, variant: ShortVariant | undefined): ShortStyle {
  const look: ShortStyle = { ...style, variants: [] }
  if (variant === undefined) return look
  if (variant.accent != null) look.accent = variant.accent
  if (variant.second != null) look.second = variant.second
  if (variant.sign_art != null) {
    // A sign of its own blinks with its own eyes, or not at all.
    look.sign = { ...look.sign, art: variant.sign_art, blink: variant.sign_blink ?? '' }
  } else if (variant.sign_blink != null) {
    look.sign = { ...look.sign, blink: variant.sign_blink }
  }
  if (variant.ground != null) {
    look.ground = {
      ...look.ground,
      pattern: variant.ground.pattern,
      seed: variant.ground.seed,
      blots: variant.ground.blots,
    }
  }
  if (variant.stamp_angle != null) look.stamp = { ...look.stamp, angle: variant.stamp_angle }
  if (variant.stamp_x != null) look.stamp = { ...look.stamp, x: variant.stamp_x }
  if (variant.wave != null) look.wave = { ...look.wave, kind: variant.wave }
  return look
}

/** A key no variant of the style has yet: `variant-2`, `variant-3`. */
export function freeKey(style: ShortStyle): string {
  const taken = new Set(style.variants.map((variant) => variant.key))
  let n = style.variants.length + 1
  while (taken.has(`variant-${String(n)}`)) n += 1
  return `variant-${String(n)}`
}

/** Whether a text is a colour as the style keeps one: `#RRGGBB`. */
export function isColour(text: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(text)
}

/** What the system's colour picker is shown while the text is not a colour
 *  yet: it takes nothing else. */
export function pickerValue(text: string): string {
  return isColour(text) ? text : '#000000'
}
