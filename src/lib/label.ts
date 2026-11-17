import type { Label } from '@/lib/api/types'

/*
 * A word of a profile's vocabulary is one string, or a map from language to
 * string (`{ en, ru }`) when a shipped profile carries it in both. This file is
 * the one place that turns either into text, with no dependency on the running
 * i18n instance, so that i18n itself can use it: a map handed to `t()` as an
 * interpolation value is resolved there instead of reaching the screen as
 * "[object Object]" - which is what it did in the score advice, the tier
 * ruler and the header menu before, and what crashed the release fields
 * outright where a map was rendered as a React child.
 */

/** Whether a value is a language map rather than plain text. */
export function isLabelMap(value: unknown): value is Record<string, string> {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return false
  const entries = Object.entries(value)
  return entries.length > 0 && entries.every(([, text]) => typeof text === 'string')
}

/**
 * A word in the given language: that language if the word has it, else
 * English - every shipped profile is written in it - else whatever the map
 * does hold. A string is returned exactly as typed.
 */
export function resolveLabel(label: Label | null | undefined, language: string): string {
  if (label === undefined || label === null) return ''
  if (typeof label === 'string') return label
  const found = label[language] ?? label[language.split('-')[0] ?? language] ?? label.en
  if (found !== undefined) return found
  return Object.values(label)[0] ?? ''
}

/** The language a word of a map is kept under: `ru` for a window in `ru-RU`. */
function wordKey(label: Record<string, string>, language: string): string {
  if (label[language] !== undefined) return language
  return language.split('-')[0] ?? language
}

/**
 * A word as the editor rewrites it in the window's language: a word kept in
 * several languages changes in this one and keeps the rest, and a word that
 * is one string stays one string.
 *
 * Until v0.112 the profile editor wrote the typed text over the whole label,
 * so renaming "Песня" in a Russian window threw the English word away - and
 * with it the way a carried-forward profile is told from one the owner
 * reworded.
 */
export function relabel(label: Label | null | undefined, text: string, language: string): Label {
  if (isLabelMap(label)) return { ...label, [wordKey(label, language)]: text }
  return text
}

/**
 * `relabel` for a word that may be absent - a description, a hint: emptied in
 * this language it leaves the map, and a map left with nothing is no word.
 */
export function relabelOptional(
  label: Label | null | undefined,
  text: string,
  language: string,
): Label | undefined {
  if (!isLabelMap(label)) return text === '' ? undefined : text
  if (text !== '') return { ...label, [wordKey(label, language)]: text }
  const rest = Object.fromEntries(
    Object.entries(label).filter(([key]) => key !== wordKey(label, language)),
  )
  return Object.keys(rest).length === 0 ? undefined : rest
}
