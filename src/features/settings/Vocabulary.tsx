import type { Kind } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'

interface VocabularyProps<T extends Kind> {
  label: string
  /** What the words are for, under them. */
  help?: string
  entries: T[]
  onChange: (entries: T[]) => void
}

/**
 * A list of the craft's words - statuses, kinds, release kinds - each renamed
 * in place. Keys come from the profile document and stay as they are.
 *
 * Generic over the entry, because a status carries a `derive` role alongside
 * its label and a kind does not — and renaming one must not drop the other.
 */
export function Vocabulary<T extends Kind>({ label, help, entries, onChange }: VocabularyProps<T>) {
  return (
    <FieldGroup label={label} help={help}>
      <ul className="flex flex-wrap gap-1.5">
        {entries.map((entry, index) => (
          <li key={entry.key} className="flex items-center gap-1">
            <Input
              className="w-40"
              value={sayLabel(entry.label)}
              onChange={(event) =>
                onChange(
                  entries.map((e, i) => (i === index ? { ...e, label: event.target.value } : e)),
                )
              }
              aria-label={entry.key}
            />
          </li>
        ))}
      </ul>
    </FieldGroup>
  )
}
