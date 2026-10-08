import { useTranslation } from 'react-i18next'
import type { TFunction } from 'i18next'
import type { ReleaseKind, WorkKind } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { FieldGroup } from '@/components/ui/field'
import { CheckedInput } from '@/features/settings/CheckedInput'

/**
 * What a kind says about the works made from others and the pictures they go
 * out with (v0.86, ADR 0047): the title a work of it takes when it is made
 * from another, the shape of each door's cover, and - read only - whether its
 * works go out under a cover and play under a frame.
 *
 * The cover and the frame are not words to rename but facts of the kind's
 * shape, like a key: the tabs a work of it has hang on them, and the place
 * to change them is the profile document.
 */
export function PublicationEditor({
  kind,
  onChange,
}: {
  kind: WorkKind
  onChange: (changes: Partial<WorkKind>) => void
}) {
  const { t } = useTranslation()
  const doors = kind.release_kinds ?? []
  // A door has a cover shape to set when its work has a picture to go out
  // with - a cover, or the frame it plays under - or when the profile gave it
  // one already, which must stay reachable to be changed or cleared.
  const pictured = kind.cover === true || kind.frame === true
  const shaped = doors.filter((door) => pictured || (door.cover_format ?? null) !== null)

  const setDoor = (key: string, changes: Partial<ReleaseKind>) =>
    onChange({
      release_kinds: doors.map((door) => (door.key === key ? { ...door, ...changes } : door)),
    })

  return (
    <>
      <CheckedInput
        label={t('editor.madeTitle')}
        help={t('editor.madeTitleHint')}
        value={sayLabel(kind.made_title)}
        check={(text) => madeTitleProblem(text, t)}
        // One word as typed, the way the label editors write one: a word the
        // author typed is theirs in whatever language they typed it. Empty
        // removes it, and a made work keeps its source's title.
        onCommit={(text) => onChange({ made_title: text === '' ? null : text })}
      />

      {shaped.length > 0 && (
        <FieldGroup label={t('editor.coverFormats')} help={t('editor.coverFormatsHint')}>
          <ul className="flex flex-col gap-1.5">
            {shaped.map((door) => (
              <li key={door.key}>
                <CheckedInput
                  // The key beside the word, as the release fields show it,
                  // and out of the box's name: a key is read, not heard.
                  label={
                    <>
                      <code aria-hidden className="mr-2 font-mono normal-case">
                        {door.key}
                      </code>
                      {sayLabel(door.label)}
                    </>
                  }
                  className="w-28"
                  placeholder="16:9"
                  value={door.cover_format ?? ''}
                  check={(text) =>
                    text === '' || isCoverFormat(text) ? null : t('editor.coverFormatInvalid')
                  }
                  onCommit={(text) =>
                    setDoor(door.key, { cover_format: text === '' ? null : text })
                  }
                />
              </li>
            ))}
          </ul>
        </FieldGroup>
      )}

      {kind.cover === true && <p className="text-xs text-dim">{t('editor.coverLine')}</p>}
      {kind.frame === true && <p className="text-xs text-dim">{t('editor.frameLine')}</p>}
    </>
  )
}

/** Two whole numbers above zero with a colon between them, as the profile
 *  checks a cover's shape: `16:9`, `1:1`. */
function isCoverFormat(text: string): boolean {
  const sides = text.split(':')
  return (
    sides.length === 2 &&
    sides.every((side) => /^\d+$/.test(side) && Number(side) > 0 && Number(side) <= 0xffffffff)
  )
}

/**
 * What the profile would refuse in a made title: one that never reads
 * `{title}` names every work made from every source the same, and a
 * placeholder other than `{title}` and `{n}` has nothing to fill it.
 */
function madeTitleProblem(text: string, t: TFunction): string | null {
  if (text === '') return null
  if (!text.includes('{title}')) return t('editor.madeTitleNoTitle')
  for (const [, name] of text.matchAll(/\{([a-z][\w:-]*)\}/g)) {
    if (name !== 'title' && name !== 'n') return t('editor.madeTitleUnknown', { name: `{${name}}` })
  }
  return null
}
