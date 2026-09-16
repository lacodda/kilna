import { describe, expect, it } from 'vitest'
import type { ReleaseFieldType } from '@/lib/api'
import en from '@/i18n/locales/en.json'
import ru from '@/i18n/locales/ru.json'

/*
 * The profile editor names a field's shape with a key it builds at runtime -
 * `editor.fieldType.${field.type}` - and the locale gate only sees keys
 * written out literally. A shape added without its word would show the key
 * itself on the screen, in both languages, and nothing would have failed.
 *
 * The same second gate `storyboard.locale.test.ts` put on the complaint
 * kinds, for the same reason.
 */

const TYPES: ReleaseFieldType[] = ['line', 'text', 'tags']

// A shape added to the union without being added here is a compile error, so
// the list below cannot silently fall behind the type.
const _every: Record<ReleaseFieldType, true> = {
  line: true,
  text: true,
  tags: true,
}
void _every

describe('every shape of release field has a word', () => {
  for (const [language, bundle] of Object.entries({ en, ru })) {
    it(`names all of them in ${language}`, () => {
      const words = (bundle as { editor: { fieldType?: Record<string, string> } }).editor.fieldType

      expect(words, `${language} has no editor.fieldType at all`).toBeDefined()
      for (const type of TYPES) {
        expect(words?.[type], `${language} says nothing for \`${type}\``).toBeTruthy()
      }
    })

    it(`names no shape that does not exist in ${language}`, () => {
      const words = (bundle as { editor: { fieldType?: Record<string, string> } }).editor.fieldType

      expect(Object.keys(words ?? {}).sort()).toEqual([...TYPES].sort())
    })
  }
})
