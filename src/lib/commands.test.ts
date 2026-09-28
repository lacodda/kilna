import { describe, expect, it } from 'vitest'
import { matchCommands } from '@/lib/commands'

const named = (...labels: string[]) => labels.map((label) => ({ label }))
const labels = (list: { label: string }[]) => list.map((entry) => entry.label)

const COMMANDS = named(
  'New work',
  'Switch to the dark theme',
  'Switch to the light theme',
  'Critical notes',
  'Calendar',
  'Score “Harbour Lights”',
)

describe('matchCommands', () => {
  it('offers everything, in the order given, for an empty query', () => {
    expect(labels(matchCommands('', COMMANDS))).toEqual(labels(COMMANDS))
    expect(labels(matchCommands('   ', COMMANDS))).toEqual(labels(COMMANDS))
  })

  it('puts a label that begins with the query before one that only contains it', () => {
    expect(labels(matchCommands('cal', COMMANDS))).toEqual(['Calendar', 'Critical notes'])
  })

  it('puts a label whose words begin with the terms before one that contains them', () => {
    const list = named('Recall the note', 'Call the note')
    expect(labels(matchCommands('note cal', list))).toEqual(['Call the note', 'Recall the note'])
  })

  it('finds every term, in any order, and ignores case', () => {
    expect(labels(matchCommands('THEME dark', COMMANDS))).toEqual(['Switch to the dark theme'])
  })

  it('drops a command missing any of the terms', () => {
    expect(matchCommands('dark work', COMMANDS)).toEqual([])
  })

  it('reads a title inside quotes as words', () => {
    expect(labels(matchCommands('harb', COMMANDS))).toEqual(['Score “Harbour Lights”'])
  })

  it('matches the words of any language the labels are in', () => {
    const list = named('Включить тёмную тему', 'Новое произведение')
    expect(labels(matchCommands('тём', list))).toEqual(['Включить тёмную тему'])
  })

  it('keeps equally good matches in the order given', () => {
    expect(labels(matchCommands('switch', COMMANDS))).toEqual([
      'Switch to the dark theme',
      'Switch to the light theme',
    ])
  })
})
