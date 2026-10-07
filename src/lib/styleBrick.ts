import type { CSSProperties } from 'react'
import type { StyleBrick, StyleType } from '@/lib/api/types'
import { say } from '@/lib/useProfile'

/*
 * What the window reads off a style brick beyond its fields: the name in the
 * window's language, the live sample of a lettering brick, the slots of a
 * dressing. Pure, so they are tested rather than looked at.
 */

/**
 * A brick's name as the window shows it: the shipped word in the window's
 * language while the brick carries one - a brick of the starter set nobody
 * renamed - and the name itself otherwise.
 */
export function styleName(brick: Pick<StyleBrick, 'name' | 'label'>): string {
  return brick.label === null ? brick.name : say(brick.label)
}

/** `#RRGGBB`, the one spelling of a colour the backend keeps. */
export const HEX = /^#[0-9a-f]{6}$/i

/** What a colour picker shows while the brick has no colour yet. */
export const NO_COLOUR = '#000000'

/** The forms whose bricks are their colours: a ground and an accent (v0.90.3). */
export function isColourForm(form: StyleType['form'] | undefined): boolean {
  return form === 'colour' || form === 'accent'
}

/**
 * How a ground or an accent is painted: its one colour, or the gradient its
 * colours make in their order - the way the prompt says it. Only colours
 * spelt as the backend keeps them; `undefined` when none is.
 */
export function colourFill(colours: readonly string[]): string | undefined {
  const kept = colours.filter((colour) => HEX.test(colour))
  if (kept.length === 0) return undefined
  if (kept.length === 1) return kept[0]
  return `linear-gradient(135deg, ${kept.join(', ')})`
}

/** The most stops a ground or an accent holds: a gradient of four. */
export const MAX_STOPS = 4

/** Paper, the ground a lettering sample is drawn on unless it names one. */
const PAPER = '#EFEBE3'
const INK = '#121114'

/**
 * The ground of a lettering sample and the ink a sample without its own
 * colour is drawn in. The sample's colours are part of the brick - a red
 * stamp, a chrome glow - designed on a ground of its own, so the ground does
 * not follow the window's theme: a dark ink on the dark theme's surface is a
 * sample nobody can see. The brick's first colour is its ground; the ink is
 * whichever of paper and ink reads on it.
 */
export function sampleGround(colours: readonly string[]): CSSProperties {
  const ground = colours[0] !== undefined && HEX.test(colours[0]) ? colours[0] : PAPER
  return { background: ground, color: lightness(ground) > 0.5 ? INK : PAPER }
}

/** Relative lightness of `#RRGGBB`, 0 to 1 - enough to pick an ink. */
function lightness(hex: string): number {
  const n = Number.parseInt(hex.slice(1), 16)
  return (0.299 * ((n >> 16) & 255) + 0.587 * ((n >> 8) & 255) + 0.114 * (n & 255)) / 255
}

/**
 * The CSS declarations a lettering brick keeps for its sample, as a style
 * object: `font-family: 'Forum'; letter-spacing: .3em` becomes
 * `{ fontFamily: "'Forum'", letterSpacing: '.3em' }`. A declaration without a
 * colon is skipped rather than guessed at; the sample is for the eye only.
 */
export function sampleStyle(css: string | null | undefined): CSSProperties {
  const style: Record<string, string> = {}
  for (const declaration of splitDeclarations(css ?? '')) {
    const colon = declaration.indexOf(':')
    if (colon <= 0) continue
    const property = declaration.slice(0, colon).trim()
    const value = declaration.slice(colon + 1).trim()
    if (property === '' || value === '') continue
    style[camel(property)] = value
  }
  return style as CSSProperties
}

/** Split at the semicolons that end a declaration, not the ones inside `url(...)` or quotes. */
function splitDeclarations(css: string): string[] {
  const out: string[] = []
  let depth = 0
  let quote: string | null = null
  let current = ''
  for (const c of css) {
    if (quote !== null) {
      if (c === quote) quote = null
    } else if (c === '"' || c === "'") quote = c
    else if (c === '(') depth += 1
    else if (c === ')') depth = Math.max(0, depth - 1)
    else if (c === ';' && depth === 0) {
      out.push(current)
      current = ''
      continue
    }
    current += c
  }
  out.push(current)
  return out
}

/** `-webkit-text-stroke` → `WebkitTextStroke`, `letter-spacing` → `letterSpacing`. */
function camel(property: string): string {
  const parts = property.replace(/^-/, '').split('-')
  const head = property.startsWith('-') ? capital(parts[0] ?? '') : (parts[0] ?? '')
  return head + parts.slice(1).map(capital).join('')
}

const capital = (word: string) => word.charAt(0).toUpperCase() + word.slice(1)

/**
 * The slots a dressing's description asks for, in the order they first
 * appear: `{brand}` and `{micro.0}` name `brand` and `micro`. The rule the
 * backend fills them by (`style_set::fill`) - a letter first, then letters,
 * digits, `_` and `-`, and an optional index.
 */
export function slotsOf(text: string): string[] {
  const out: string[] = []
  for (const match of text.matchAll(/\{([A-Za-z][A-Za-z0-9_-]*)(?:\.\d+)?\}/g)) {
    const name = match[1]
    if (name !== undefined && !out.includes(name)) out.push(name)
  }
  return out
}

/** The types a new brick may be made in: every one the profile has not retired. */
export function livingTypes(types: StyleType[]): StyleType[] {
  return types.filter((one) => one.retired === undefined || one.retired === null)
}
