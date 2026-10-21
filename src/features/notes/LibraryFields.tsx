import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { History } from 'lucide-react'
import { entryWorkedIn } from '@/lib/api/boards'
import type { Note, NoteKind, NotePatch } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { say as sayLabel } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/AppSelect'
import { useFieldDraft } from '@/components/ui/field-draft'

interface Props {
  note: Note
  /** The library's kind: the media an entry can be in. */
  library: NoteKind
  onPatch: (patch: NotePatch) => void
}

/**
 * What an entry of the library is (v0.100, ADR 0066): who made it, when, in
 * what medium, the other titles it goes by - and the works whose boards it
 * worked on. One record for the whole workspace: what is put right here is
 * right on every board that links to it.
 */
export function LibraryFields({ note, library, onPatch }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const worked = useQuery({
    queryKey: keys.entryWorkedIn(note.id),
    queryFn: () => entryWorkedIn(note.id),
    staleTime: 0,
  })
  const author = useFieldDraft(note.attribution ?? '', (text) => {
    if (text.trim() !== (note.attribution ?? '')) onPatch({ attribution: text.trim() || null })
  })
  const year = useFieldDraft(note.year ?? '', (text) => {
    if (text.trim() !== (note.year ?? '')) onPatch({ year: text.trim() || null })
  })
  const aliases = useFieldDraft(note.aliases.join('; '), (text) => {
    const next = text
      .split(';')
      .map((alias) => alias.trim())
      .filter((alias) => alias !== '')
    if (next.join('; ') !== note.aliases.join('; ')) onPatch({ aliases: next })
  })

  return (
    <div className="flex shrink-0 flex-col gap-2 border-b border-line px-3 py-2">
      <div className="grid grid-cols-[minmax(0,2fr)_minmax(0,1fr)_minmax(0,1fr)] gap-2 max-[900px]:grid-cols-1">
        <label className="flex flex-col gap-0.5">
          <span className="caption text-2xs">{t('library.author')}</span>
          <Input {...author} placeholder={t('library.authorPlaceholder')} />
        </label>
        <label className="flex flex-col gap-0.5">
          <span className="caption text-2xs">{t('library.year')}</span>
          <Input {...year} placeholder="2006" />
        </label>
        <div className="flex flex-col gap-0.5">
          <span className="caption text-2xs">{t('library.medium')}</span>
          <Select
            value={note.medium ?? ''}
            placeholder={t('library.noMedium')}
            onChange={(next) => {
              if (next !== (note.medium ?? '')) onPatch({ medium: next === '' ? null : next })
            }}
            options={(library.media ?? []).map((medium) => ({
              value: medium.key,
              label: sayLabel(medium.label),
            }))}
            aria-label={t('library.medium')}
          />
        </div>
      </div>
      <label className="flex flex-col gap-0.5">
        <span className="caption text-2xs">{t('library.aliases')}</span>
        <Input {...aliases} placeholder={t('library.aliasesPlaceholder')} />
      </label>
      <p className="flex flex-wrap items-center gap-1 text-xs text-dim">
        <History aria-hidden className="size-3.5 text-faint" />
        {(worked.data ?? []).length === 0 ? (
          t('library.workedNowhere')
        ) : (
          <>
            {t('library.workedIn')}
            {(worked.data ?? []).map((work) => (
              <Button
                key={work.work_id}
                variant="link"
                className="h-auto p-0 text-xs"
                onClick={() => void navigate(`/works/${work.work_id}/conception`)}
              >
                «{work.title}»
              </Button>
            ))}
          </>
        )}
      </p>
    </div>
  )
}
