import { stem } from '@/lib/stem'

/**
 * Words a text uses more than once, and where.
 *
 * Written into a song, "лестница" three times in different cases is one image
 * leaned on three times, and the writer would rather know it before a
 * listener does. The counter groups the words of a text by their light stem
 * (`lib/stem`), keeps the groups that occur twice or more, and hands back the
 * places so the editor can mark them.
 *
 * Function words are left out: "и", "the" and "не" repeat in every text ever
 * written and a page lit up by them would say nothing. So are stems shorter
 * than three letters, and anything inside square brackets - `[Verse 2]` names
 * a section and repeats by design.
 */

export interface RepeatGroup {
  /** The grouping key. Not for showing. */
  stem: string
  /** The word as it first appears, for showing. */
  word: string
  count: number
}

export interface RepeatMark {
  start: number
  end: number
  /** Index into the groups, most repeated first. */
  group: number
}

export interface Repeats {
  groups: RepeatGroup[]
  marks: RepeatMark[]
}

/** Stems shorter than this are pronouns, particles and noise. */
const MIN_STEM = 3

const STOP = new Set(
  (
    'и в во не что он на я с со как а то все всё она так его но да ты к у же вы за бы по только ее её ' +
    'мне было вот от меня еще ещё нет о из ему теперь когда даже ну вдруг ли если уже или ни быть был ' +
    'него до вас нибудь опять уж вам ведь там потом себя ничего ей может они тут где есть надо ней для ' +
    'мы тебя их чем была сам чтоб без будто чего раз тоже себе под будет ж тогда кто этот того потому ' +
    'этого какой совсем ним здесь этом один почти мой тем чтобы нее неё сейчас были куда зачем всех ' +
    'никогда можно при наконец два об другой хоть после над больше тот через эти нас про всего них ' +
    'какая много разве три эту моя впрочем хорошо свою этой перед иногда лучше чуть том нельзя такой ' +
    'им более всегда конечно всю между это эта было будут буду мою твой твоя твою свой своя своё их ' +
    'the a an and or but of to in on at by for with from as is are was were be been being it its this ' +
    'that these those i you he she we they me him her us them my your his our their not no so if then ' +
    'than too very just do does did have has had will would can could should may might there here what ' +
    'which who whom when where why how all any each every some more most other into out up down over ' +
    'under again off only own same am oh yeah ll ve re'
  ).split(' '),
)

const WORD = /[\p{L}\p{M}]+(?:['’-][\p{L}\p{M}]+)*/gu

/** The spans of `[...]` on each line: section labels, not lyrics. */
function bracketed(text: string): [number, number][] {
  const spans: [number, number][] = []
  const pattern = /\[[^\]\n]*\]/g
  for (const hit of text.matchAll(pattern)) {
    spans.push([hit.index, hit.index + hit[0].length])
  }
  return spans
}

export function findRepeats(text: string): Repeats {
  const skip = bracketed(text)
  const inBrackets = (at: number) => skip.some(([start, end]) => at >= start && at < end)

  interface Seen {
    word: string
    places: [number, number][]
  }
  const seen = new Map<string, Seen>()

  for (const hit of text.matchAll(WORD)) {
    if (inBrackets(hit.index)) continue
    const word = hit[0]
    const lower = word.toLowerCase().replace(/ё/g, 'е')
    if (STOP.has(lower)) continue
    const key = stem(word)
    if (key.length < MIN_STEM) continue

    const entry = seen.get(key) ?? { word, places: [] }
    entry.places.push([hit.index, hit.index + word.length])
    seen.set(key, entry)
  }

  const repeated = [...seen.entries()]
    .filter(([, entry]) => entry.places.length > 1)
    // Most repeated first; equal counts in order of first appearance, which
    // is the order the map already holds.
    .sort((a, b) => b[1].places.length - a[1].places.length)

  const groups: RepeatGroup[] = repeated.map(([key, entry]) => ({
    stem: key,
    word: entry.word,
    count: entry.places.length,
  }))
  const marks: RepeatMark[] = repeated
    .flatMap(([, entry], group) => entry.places.map(([start, end]) => ({ start, end, group })))
    .sort((a, b) => a.start - b.start)

  return { groups, marks }
}
