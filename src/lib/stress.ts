import type { StressKind, StressNote } from '@/lib/api/types'
import { cn } from '@/lib/utils'

/*
 * A text that is sung, as the window writes it (ADR 0053).
 *
 * The owner writes a lyric for a singer that reads letters, not intentions,
 * so the stress goes into the text itself: a capital vowel past a word's
 * first letter ("пульсАр"), when it is the word's only capital there; or a
 * combining acute after the vowel ("О́блако"), which is how a first letter -
 * a capital already at the start of a line - is marked. Ё is always
 * stressed. Two or more capitals past the first letter are an abbreviation
 * or a shout, never a mark.
 *
 * Reading those marks and checking a text against the dictionary is the
 * backend's (`words/sung.rs`): one analyser for every text, so the strip and
 * the marks never disagree with what a save is checked against. What lives
 * here is only the writing - the gesture that puts a mark on a vowel or takes
 * it off, and the answer to a note written into the run it names - because
 * that happens at the caret, between two keystrokes, where a round trip to
 * the backend would be felt. The word is cut the way the backend cuts it, so
 * "the same word" means the same thing on both sides.
 *
 * Offsets are UTF-16 units throughout: the textarea's, the backend's notes',
 * and JavaScript's string indices are all the same count.
 */

/** The combining acute: a stress written over the letter before it. */
export const ACUTE = '́'

const VOWELS = new Set('аеёиоуыэюя')

/** Whether a letter is a Russian vowel, in either case. Marks are Russian:
 *  a capital inside "LinkedIn" is the brand's, and the backend reads no
 *  stress into a word that is not Cyrillic. */
function isVowel(letter: string): boolean {
  return VOWELS.has(letter.toLowerCase())
}

/** A combining mark, U+0300-U+036F: a stress written over a letter. */
function isMark(char: string): boolean {
  const code = char.charCodeAt(0)
  return code >= 0x300 && code <= 0x36f
}

function isLetter(char: string): boolean {
  return char !== '' && (/\p{L}/u.test(char) || isMark(char))
}

function isUpper(letter: string): boolean {
  return letter !== letter.toLowerCase() && letter === letter.toUpperCase()
}

/** A run of the text, in UTF-16 units: `start` included, `end` not. */
export interface Span {
  start: number
  end: number
}

/**
 * Every word of a text, cut the way the backend cuts it (`words::words`):
 * letters and the marks over them, an apostrophe or a hyphen joining two runs
 * of letters - "из-за" is one word, a trailing hyphen is punctuation.
 */
export function wordsOf(text: string): Span[] {
  const out: Span[] = []
  let at = 0
  while (at < text.length) {
    if (!isLetter(text.charAt(at))) {
      at += 1
      continue
    }
    const start = at
    at += 1
    for (;;) {
      while (at < text.length && isLetter(text.charAt(at))) at += 1
      const joiner = text.charAt(at)
      const joins = joiner !== '' && "'’-".includes(joiner) && isLetter(text.charAt(at + 1))
      if (!joins) break
      at += 1
    }
    out.push({ start, end: at })
  }
  return out
}

/** The word the character at `index` belongs to, if it belongs to one. */
export function wordAt(text: string, index: number): Span | null {
  if (index < 0 || index >= text.length) return null
  return wordsOf(text).find((word) => word.start <= index && index < word.end) ?? null
}

/** One letter of a word with the marks written over it. */
interface Letter {
  char: string
  /** Where it stands from the word's start. */
  at: number
  /** The combining marks after it, the acute among them. */
  tail: string
}

function lettersOf(word: string): Letter[] {
  const out: Letter[] = []
  for (let at = 0; at < word.length; at += 1) {
    const char = word.charAt(at)
    const last = out[out.length - 1]
    if (isMark(char) && last !== undefined) last.tail += char
    else out.push({ char, at, tail: '' })
  }
  return out
}

const withoutAcute = (tail: string) => tail.split(ACUTE).join('')

/** The capitals past the first letter, by their place among the letters. */
function capitals(letters: readonly Letter[]): number[] {
  const out: number[] = []
  letters.forEach((letter, place) => {
    if (place > 0 && isUpper(letter.char)) out.push(place)
  })
  return out
}

/** Which letter the word marks as stressed, and how - the backend's
 *  `marked`, in the same order: an acute first, then a lone capital vowel
 *  past the first letter, then ё. */
function markOf(
  letters: readonly Letter[],
): { place: number; by: 'acute' | 'capital' | 'yo' } | null {
  const acute = letters.findIndex((letter) => isVowel(letter.char) && letter.tail.includes(ACUTE))
  if (acute >= 0) return { place: acute, by: 'acute' }
  const [only, ...more] = capitals(letters)
  if (only !== undefined && more.length === 0 && isVowel(letters[only]!.char)) {
    return { place: only, by: 'capital' }
  }
  const yo = letters.findIndex((letter) => letter.char === 'ё' || letter.char === 'Ё')
  return yo >= 0 ? { place: yo, by: 'yo' } : null
}

/** Every stress mark taken off: the acutes dropped, a lone capital vowel past
 *  the first letter lowered. A first capital, an abbreviation and a shout
 *  stay as they are; so does ё, which is spelling before it is a mark. */
function unmark(letters: Letter[]): void {
  for (const letter of letters) letter.tail = withoutAcute(letter.tail)
  const [only, ...more] = capitals(letters)
  if (only === undefined || more.length > 0) return
  const capital = letters[only]!
  if (isVowel(capital.char)) capital.char = capital.char.toLowerCase()
}

/**
 * The stress on the vowel at `place` toggled, in place.
 *
 * Unmarked, it gets the mark and every other mark in the word goes, so the
 * word says one thing: a capital, or the acute when the vowel is the first
 * letter (whose capital means the start of a line) or when the word already
 * has a capital past its first letter (a second one would turn the mark into
 * an abbreviation). Marked, the mark comes off. "е" walks one step further,
 * the way the owner hears it: е, then Е (stressed), then ё, then е again.
 */
function toggle(letters: Letter[], place: number): void {
  const target = letters[place]!
  const lower = target.char.toLowerCase()
  if (lower === 'ё') {
    target.char = target.char === 'Ё' ? 'Е' : 'е'
    target.tail = withoutAcute(target.tail)
    return
  }
  const mark = markOf(letters)
  if (mark?.place === place) {
    if (mark.by === 'acute') {
      // The letter under an acute keeps its case: a first capital is the
      // start of a line, and a shout is a shout.
      target.tail = withoutAcute(target.tail)
      if (lower === 'е') target.char = isUpper(target.char) ? 'Ё' : 'ё'
    } else {
      target.char = lower === 'е' ? 'ё' : lower
    }
    return
  }
  unmark(letters)
  if (place > 0 && capitals(letters).length === 0) target.char = target.char.toUpperCase()
  else target.tail = ACUTE + target.tail
}

/** A text after an edit, and where a caret that was somewhere in it is now. */
export interface Edited {
  text: string
  caret: number
}

/**
 * The stress on the vowel at `index` toggled (see `toggle`), with `caret`
 * carried over the edit: a mark added or taken off before it moves it by one,
 * and a caret that sat between a letter and its marks lands after them, so it
 * never splits a vowel from its acute. Null when `index` is not a vowel of a
 * word - the gesture then does nothing rather than something unexpected.
 */
export function toggleStress(text: string, index: number, caret: number = index): Edited | null {
  const word = wordAt(text, index)
  if (word === null) return null
  const before = lettersOf(text.slice(word.start, word.end))
  const place = before.findIndex(
    (letter) =>
      index - word.start >= letter.at && index - word.start < letter.at + 1 + letter.tail.length,
  )
  if (place < 0 || !isVowel(before[place]!.char)) return null

  const after = before.map((letter) => ({ ...letter }))
  toggle(after, place)
  const written = after.map((letter) => letter.char + letter.tail).join('')

  // Where the caret goes, letter by letter: the same letter, its marks now
  // as long as they are.
  const carried = (() => {
    if (caret <= word.start) return caret
    if (caret >= word.end) return caret + written.length - (word.end - word.start)
    const offset = caret - word.start
    let at = 0
    for (let i = 0; i < before.length; i += 1) {
      const old = before[i]!
      const length = 1 + after[i]!.tail.length
      if (offset === old.at) return word.start + at
      if (offset < old.at + 1 + old.tail.length) return word.start + at + length
      at += length
    }
    return word.start + at
  })()

  return {
    text: text.slice(0, word.start) + written + text.slice(word.end),
    caret: carried,
  }
}

/**
 * The vowel at `index`, or the one a combining mark at `index` sits over;
 * null for anything else. A click lands between two characters, and the
 * caller says which of them it meant.
 */
export function vowelAt(text: string, index: number): number | null {
  let at = index
  while (at > 0 && isMark(text.charAt(at))) at -= 1
  if (at < 0 || at >= text.length || !isVowel(text.charAt(at))) return null
  return wordAt(text, at) === null ? null : at
}

/**
 * The vowel the keyboard's stress shortcut means at `caret`: in the word the
 * caret stands in or right after, the nearest vowel before the caret - the
 * one just typed, which is how a word is marked while it is written
 * ("замо", the shortcut, "замОк") - or, at the word's start, its first.
 */
export function vowelNear(text: string, caret: number): number | null {
  const word = wordAt(text, caret - 1) ?? wordAt(text, caret)
  if (word === null) return null
  for (let at = Math.min(caret, word.end) - 1; at >= word.start; at -= 1) {
    if (isVowel(text.charAt(at))) return at
  }
  for (let at = Math.max(caret, word.start); at < word.end; at += 1) {
    if (isVowel(text.charAt(at))) return at
  }
  return null
}

/** `form` with the case of `like`'s first letter. The backend writes a
 *  stress on a first letter as a capital with an acute ("А́тлас"); in the
 *  middle of a line that capital would reach the public text, where the
 *  clean copy drops only the acute. The word on the page keeps its case. */
function casedLike(form: string, like: string): string {
  const first = form.charAt(0)
  const rest = form.slice(1)
  return (isUpper(like.charAt(0)) ? first.toUpperCase() : first.toLowerCase()) + rest
}

/**
 * The text with `option` written over the run `note` names - exactly that
 * run, and only while it still holds the word the note was about. A note is
 * an answer to the text as it was asked about; once the text moved under it,
 * writing at its offsets would land on another word, and nothing is written.
 */
export function answer(text: string, note: StressNote, option: string): string | null {
  if (text.slice(note.start, note.end) !== note.word) return null
  return text.slice(0, note.start) + casedLike(option, note.word) + text.slice(note.end)
}

/** Whether a note has one answer and nothing to choose: the ё spelling, the
 *  dictionary's one stress, the owner's own respelling. A homograph is a
 *  choice whatever its count, and a word nobody knows has no answer. */
export function hasOneAnswer(note: StressNote): boolean {
  return note.kind !== 'homograph' && note.kind !== 'unknown' && note.options.length === 1
}

/** A run of the text written over: where it was, and how long what was
 *  written there is. */
export interface Replaced extends Span {
  length: number
}

/**
 * Every note of `notes` written over with what `pick` answers for it, the last
 * first so the offsets of the ones before stay true. A word can carry two
 * notes - a missing ё and a stress against the dictionary - over the same
 * run; the first written wins and the other waits for the next check, which
 * reads the word as it now is.
 */
export function answerEach(
  text: string,
  notes: readonly StressNote[],
  pick: (note: StressNote) => string | undefined,
): { text: string; replaced: Replaced[] } {
  let out = text
  let floor = Number.POSITIVE_INFINITY
  const replaced: Replaced[] = []
  const latestFirst = [...notes].sort((a, b) => b.start - a.start)
  for (const note of latestFirst) {
    if (note.end > floor) continue
    const option = pick(note)
    if (option === undefined) continue
    const next = answer(out, note, option)
    if (next === null) continue
    out = next
    floor = note.start
    replaced.unshift({ start: note.start, end: note.end, length: option.length })
  }
  return { text: out, replaced }
}

/** Every note with one answer written into the text at once. */
export function answerAll(
  text: string,
  notes: readonly StressNote[],
): { text: string; replaced: Replaced[] } {
  return answerEach(text, notes.filter(hasOneAnswer), (note) => note.options[0])
}

/** The notes on one word of the strip: every place a text writes it, or -
 *  for a homograph - the one place, since each is its own choice. */
export interface NoteGroup {
  key: string
  notes: StressNote[]
}

/**
 * The notes as the strip shows them, in the order of the text: a word the
 * dictionary does not know, a missing ё, a stress against the dictionary is
 * one entry however many times the text says it - "Lisbon" six times is one
 * question - and an answer goes to every place. A homograph is an entry a
 * place: "замок" in one line and in the next may be two words.
 */
export function groupNotes(notes: readonly StressNote[]): NoteGroup[] {
  const groups: NoteGroup[] = []
  const byWord = new Map<string, NoteGroup>()
  for (const note of [...notes].sort((a, b) => a.start - b.start)) {
    if (note.kind === 'homograph') {
      groups.push({ key: `${note.start}:${note.kind}`, notes: [note] })
      continue
    }
    const key = `${note.kind}:${note.word.toLowerCase()}`
    const known = byWord.get(key)
    if (known !== undefined) {
      known.notes.push(note)
      continue
    }
    const group = { key, notes: [note] }
    byWord.set(key, group)
    groups.push(group)
  }
  return groups
}

/**
 * Where a caret stands once runs before it were written over (in text order,
 * none overlapping): moved by how much each grew or shrank, and at the end
 * of a run it stood inside - a caret in the middle of a word that changed
 * has no letter left to stand beside.
 */
export function carryCaret(caret: number, replaced: readonly Replaced[]): number {
  let shift = 0
  for (const run of replaced) {
    if (caret >= run.end) shift += run.length - (run.end - run.start)
    else if (caret > run.start) return run.start + shift + run.length
    else break
  }
  return caret + shift
}

/** How a note of this kind is marked in a text: see `styles.css`. */
export function stressMark(kind: StressKind): string {
  return `stress-${kind}`
}

/** A run of marked text: what `MarkedText` draws. */
export interface Run extends Span {
  className?: string
}

/**
 * The marks of a text with an accent over each stressed vowel ("show the
 * stresses"), cut so that no two runs overlap - the marked text draws runs
 * in order and a run inside another would draw its letters twice.
 *
 * A vowel inside a marked word splits the word's run in three, and the cut
 * edges are told so (`mark-open-*`), so a tinted word stays one wash rather
 * than three rounded pieces. No accent is drawn over ё, whose dots already
 * say it, nor over a vowel with an acute already written on it.
 */
export function withAccents(runs: readonly Run[], accents: readonly number[], text: string): Run[] {
  const points = accents.filter((at) => {
    const letter = text.charAt(at)
    return isVowel(letter) && letter.toLowerCase() !== 'ё' && text.charAt(at + 1) !== ACUTE
  })
  if (points.length === 0) return [...runs]
  const accent = (at: number) =>
    cn('stress-accent', isUpper(text.charAt(at)) && 'stress-accent-cap')

  const out: Run[] = []
  const used = new Set<number>()
  for (const run of runs) {
    const inside = points.filter((at) => at >= run.start && at < run.end)
    if (inside.length === 0) {
      out.push(run)
      continue
    }
    let from = run.start
    for (const at of inside) {
      used.add(at)
      if (at > from) {
        out.push({
          start: from,
          end: at,
          className: cn(run.className, from > run.start && 'mark-open-start', 'mark-open-end'),
        })
      }
      out.push({
        start: at,
        end: at + 1,
        className: cn(
          run.className,
          accent(at),
          at > run.start && 'mark-open-start',
          at + 1 < run.end && 'mark-open-end',
        ),
      })
      from = at + 1
    }
    if (from < run.end) {
      out.push({ start: from, end: run.end, className: cn(run.className, 'mark-open-start') })
    }
  }
  for (const at of points) {
    if (!used.has(at)) out.push({ start: at, end: at + 1, className: accent(at) })
  }
  return out.sort((a, b) => a.start - b.start)
}
