/**
 * A light stemmer for Russian and English words.
 *
 * Enough to say that "лестница", "лестницы" and "лестницей" are one word, and
 * that "ladder" and "ladders" are - which is the whole of what the repeat
 * counter needs. Not a lemmatiser: the stem is a key for grouping, never
 * shown as a word, so "лестниц" is a fine answer and "лестница" would cost a
 * dictionary. The Russian side follows the shape of the Snowball algorithm -
 * a region after the first vowel, one suffix class stripped in order - with
 * the derivational step left out; the English side is Porter's first steps.
 *
 * Own rather than a dependency: the two available packages weigh more than
 * this whole application's frontend and cover forty languages this text will
 * never be written in.
 */

const RU_VOWELS = 'аеиоуыэюя'

/** Snowball's RV: everything after the first vowel. Suffixes are only ever
 *  taken from inside it, which is what keeps "мыло" from losing its "ло". */
function rv(word: string): number {
  for (let at = 0; at < word.length; at += 1) {
    if (RU_VOWELS.includes(word[at]!)) return at + 1
  }
  return word.length
}

/** Takes the first suffix of `suffixes` the word ends with, inside RV. */
function strip(word: string, from: number, suffixes: readonly string[]): string | null {
  for (const suffix of suffixes) {
    if (word.length - suffix.length >= from && word.endsWith(suffix)) {
      return word.slice(0, word.length - suffix.length)
    }
  }
  return null
}

/** Suffixes that count only after `а` or `я`: the letter stays. */
function stripAfterAYa(word: string, from: number, suffixes: readonly string[]): string | null {
  for (const suffix of suffixes) {
    if (word.length - suffix.length - 1 >= from && word.endsWith(suffix)) {
      const before = word[word.length - suffix.length - 1]
      if (before === 'а' || before === 'я') return word.slice(0, word.length - suffix.length)
    }
  }
  return null
}

// Longest first within each class, as Snowball lists them: "ившись" must be
// tried before "ив", or the wrong one wins.
const PERFECTIVE_GERUND_AYA = ['вшись', 'вши', 'в']
const PERFECTIVE_GERUND = ['ившись', 'ывшись', 'ивши', 'ывши', 'ив', 'ыв']
const REFLEXIVE = ['ся', 'сь']
const ADJECTIVE = [
  'ими', 'ыми', 'его', 'ого', 'ему', 'ому', 'ее', 'ие', 'ые', 'ое', 'ей', 'ий', 'ый', 'ой',
  'ем', 'им', 'ым', 'ом', 'их', 'ых', 'ую', 'юю', 'ая', 'яя', 'ою', 'ею',
]
const PARTICIPLE_AYA = ['ем', 'нн', 'вш', 'ющ', 'щ']
const PARTICIPLE = ['ивш', 'ывш', 'ующ']
const VERB_AYA = [
  'ете', 'йте', 'ешь', 'нно', 'ла', 'на', 'ли', 'ем', 'ло', 'но', 'ет', 'ют', 'ны', 'ть', 'й', 'л', 'н',
]
const VERB = [
  'ейте', 'уйте', 'ила', 'ыла', 'ена', 'ите', 'или', 'ыли', 'ило', 'ыло', 'ено', 'ует', 'уют',
  'ены', 'ить', 'ыть', 'ишь', 'ей', 'уй', 'ил', 'ыл', 'им', 'ым', 'ен', 'ят', 'ит', 'ыт', 'ую', 'ю',
]
const NOUN = [
  'иями', 'ями', 'ами', 'ией', 'иям', 'ием', 'иях', 'ев', 'ов', 'ие', 'ье', 'еи', 'ии', 'ей', 'ой',
  'ий', 'ям', 'ем', 'ам', 'ом', 'ах', 'ях', 'ию', 'ью', 'ия', 'ья', 'а', 'е', 'и', 'й', 'о', 'у', 'ы',
  'ь', 'ю', 'я',
]

function stemRussian(input: string): string {
  const word = input.replace(/ё/g, 'е')
  const from = rv(word)
  let out = word

  // Step 1: one of the perfective gerund endings; failing that, the
  // reflexive ending and then one adjectival, verbal or noun ending.
  const gerund = stripAfterAYa(out, from, PERFECTIVE_GERUND_AYA) ?? strip(out, from, PERFECTIVE_GERUND)
  if (gerund !== null) {
    out = gerund
  } else {
    out = strip(out, from, REFLEXIVE) ?? out
    const adjective = strip(out, from, ADJECTIVE)
    if (adjective !== null) {
      out = adjective
      out = stripAfterAYa(out, from, PARTICIPLE_AYA) ?? strip(out, from, PARTICIPLE) ?? out
    } else {
      const verb = stripAfterAYa(out, from, VERB_AYA) ?? strip(out, from, VERB)
      out = verb ?? strip(out, from, NOUN) ?? out
    }
  }

  // Step 2: a trailing и.
  out = strip(out, from, ['и']) ?? out

  // Step 4: a trailing ь; a doubled н; the superlative.
  const superlative = strip(out, from, ['ейше', 'ейш'])
  if (superlative !== null) out = superlative
  if (out.endsWith('нн') && out.length - 1 >= from) out = out.slice(0, -1)
  out = strip(out, from, ['ь']) ?? out

  return out
}

const EN_VOWELS = 'aeiouy'

function hasVowel(word: string): boolean {
  return [...word].some((letter) => EN_VOWELS.includes(letter))
}

function stemEnglish(word: string): string {
  let out = word

  // Plurals.
  if (out.endsWith('sses')) out = out.slice(0, -2)
  else if (out.endsWith('ies') && out.length > 4) out = out.slice(0, -2)
  else if (out.endsWith('ss') || out.endsWith('us')) {
    // Stays.
  } else if (out.endsWith('s') && out.length > 3) out = out.slice(0, -1)

  // -ed, -ing: only where a vowel remains, so "sing" stays "sing".
  for (const suffix of ['ing', 'ed']) {
    if (out.endsWith(suffix) && out.length - suffix.length >= 2) {
      const stem = out.slice(0, -suffix.length)
      if (!hasVowel(stem)) continue
      // "hopping" → "hop", but "falling" → "fall" is wrong either way; the
      // undoubling rule Porter uses errs towards short stems, and a short
      // stem groups more, which is the safe direction for a repeat counter.
      if (
        stem.length >= 3 &&
        stem[stem.length - 1] === stem[stem.length - 2] &&
        !'lsz'.includes(stem[stem.length - 1]!)
      ) {
        out = stem.slice(0, -1)
      } else if (stem.endsWith('at') || stem.endsWith('bl') || stem.endsWith('iz')) {
        out = `${stem}e`
      } else {
        out = stem
      }
      break
    }
  }

  // Common derivational tails a lyric repeats through.
  for (const suffix of ['ness', 'ment', 'ful', 'ly', 'er']) {
    if (out.endsWith(suffix) && out.length - suffix.length >= 3) {
      out = out.slice(0, -suffix.length)
      break
    }
  }

  // A final y is an i: "happy" and "happier" meet at "happi".
  if (out.endsWith('y') && out.length > 2) out = `${out.slice(0, -1)}i`
  // A final e is dropped past three letters: "love" and "loving" meet at "lov".
  if (out.endsWith('e') && out.length > 3) out = out.slice(0, -1)

  return out
}

/** The grouping key of a word: its light stem, lowercased. Scripts other
 *  than Cyrillic and Latin are returned lowercased and whole. */
export function stem(word: string): string {
  const lower = word.toLowerCase()
  if (/[а-яё]/.test(lower)) return stemRussian(lower)
  if (/^[a-z'’-]+$/.test(lower)) return stemEnglish(lower.replace(/['’].*$/, ''))
  return lower
}
