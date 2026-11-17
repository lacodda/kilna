import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { open as pick } from '@tauri-apps/plugin-dialog'
import { bringIntake } from '@/lib/api/data'
import type { IntakeFound, IntakeRequest } from '@/lib/api/types'
import { sayReason } from '@/lib/errors'
import { queries } from '@/lib/query/queries'
import { say } from '@/lib/toast'
import { useDebounced } from '@/lib/useDebounced'
import { labelOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import { Dialog } from '@/components/AppDialog'
import { Select } from '@/components/AppSelect'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Called with the work's id when exactly one was brought in. */
  onBrought?: (workId: string) => void
}

/**
 * Works coming in from outside: files, a folder, or a text pasted here
 * (v0.113).
 *
 * One dialog for the three, because what a person has is "these" and what it
 * is decides itself: a page kilna wrote and a `.kilna.zip` say their own kind
 * and come back as the works they were, and any other text becomes a work of
 * the kind named here, with the text as its first version. Nothing is written
 * until the button is pressed: the list under the fields is the backend's
 * reading of what was handed over - what each is called, and whether it is
 * here already - so what the button will do is on the screen before it does
 * it.
 */
export function IntakeDialog({ open, onOpenChange, onBrought }: Props) {
  return (
    // Keyed by being open, so each opening starts empty: the files chosen
    // last time were brought in last time.
    <Body key={String(open)} open={open} onOpenChange={onOpenChange} onBrought={onBrought} />
  )
}

function Body({ open, onOpenChange, onBrought }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const profile = useProfile()
  const kinds = profile.config.work_kinds

  const [paths, setPaths] = useState<string[]>([])
  const [text, setText] = useState('')
  const [title, setTitle] = useState('')
  const [kind, setKind] = useState(kinds[0]?.key ?? '')
  const [busy, setBusy] = useState(false)

  // The text is read as it settles, not per keystroke: reading it is a round
  // trip, and a pasted page is thousands of characters.
  const settledText = useDebounced(text, 250)
  const settledTitle = useDebounced(title, 250)
  const request = requestOf(paths, settledText, settledTitle, kind)
  const asked = paths.length > 0 || request.text !== undefined
  const survey = useQuery({ ...queries.surveyIntake(request), enabled: open && asked })
  const found = asked ? (survey.data ?? []) : []

  const comes = found.filter((one) => one.standing === 'new' && one.reason === null)
  // A text from outside says no kind of its own; the choice is shown only
  // when there is one to make it for.
  const hasText = found.some((one) => one.kind === null && one.standing !== null)
  const pasted = found.find((one) => one.name === null)

  const choose = async (directory: boolean) => {
    try {
      const picked = await pick(
        directory
          ? { directory: true, title: t('intake.folderTitle') }
          : {
              multiple: true,
              title: t('intake.filesTitle'),
              filters: [
                { name: 'Markdown, text, kilna', extensions: ['md', 'markdown', 'txt', 'zip'] },
              ],
            },
      )
      if (picked === null) return
      const chosen = Array.isArray(picked) ? picked : [picked]
      setPaths((before) => [...before, ...chosen.filter((path) => !before.includes(path))])
    } catch (cause) {
      say.failed(cause)
    }
  }

  const bring = async () => {
    setBusy(true)
    try {
      // What is on the screen, not what has settled: the last key pressed
      // before the button counts.
      const brought = await bringIntake(requestOf(paths, text, title, kind))
      // Works, versions, scores, releases, files: everything read so far.
      void client.invalidateQueries()

      const said = [t('intake.brought', { count: brought.works.length })]
      if (brought.here + brought.deleted > 0) {
        said.push(t('intake.passedOver', { here: brought.here, deleted: brought.deleted }))
      }
      const missing = brought.works.reduce((sum, one) => sum + one.missing, 0)
      if (missing > 0) said.push(t('intake.missingFiles', { count: missing }))
      if (brought.refused.length > 0) {
        // Left open: the list says which, and why.
        say.warn(said.join(' '), t('intake.refused', { count: brought.refused.length }))
        return
      }
      say.ok(said.join(' '))
      onOpenChange(false)
      const [only] = brought.works
      if (brought.works.length === 1 && only !== undefined) onBrought?.(only.id)
    } catch (cause) {
      say.failedTo(t('intake.failed'), cause)
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('intake.title')}
      description={t('intake.hint')}
      size="lg"
      dirty={paths.length > 0}
      footer={
        <Button
          variant="primary"
          disabled={busy || comes.length === 0 || survey.isFetching}
          onClick={() => void bring()}
        >
          {t('intake.bring', { count: comes.length })}
        </Button>
      }
    >
      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-1.5">
          <Button onClick={() => void choose(false)}>{t('intake.files')}</Button>
          <Button onClick={() => void choose(true)}>{t('intake.folder')}</Button>
          {paths.length > 0 && (
            <Button variant="ghost" onClick={() => setPaths([])}>
              {t('intake.clear')}
            </Button>
          )}
        </div>

        <Field label={t('intake.paste')}>
          <Textarea
            value={text}
            onChange={(event) => setText(event.target.value)}
            placeholder={t('intake.pastePlaceholder')}
            rows={3}
            autoResize
            maxRows={8}
          />
        </Field>

        {pasted !== undefined && pasted.kind === null && (
          <Field label={t('intake.pastedTitle')}>
            <Input
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder={pasted.title}
            />
          </Field>
        )}

        {hasText && kinds.length > 1 && (
          <FieldGroup label={t('intake.kind')}>
            <Select
              aria-label={t('intake.kind')}
              value={kind}
              onChange={setKind}
              options={kinds.map((entry) => ({ value: entry.key, label: sayLabel(entry.label) }))}
            />
          </FieldGroup>
        )}

        {asked && survey.data !== undefined && found.length === 0 && (
          <p className="text-sm text-dim">{t('intake.noneFound')}</p>
        )}
        {found.length > 0 && (
          <ul className="flex max-h-72 flex-col overflow-y-auto rounded-md border border-line">
            {found.map((one, index) => (
              <FoundRow
                // A folder may hold two files of one name in different
                // places of the list; the place is what tells them apart.
                key={`${String(index)}:${one.name ?? ''}`}
                found={one}
                kind={one.kind ?? (kind === '' ? null : kind)}
              />
            ))}
          </ul>
        )}
      </div>
    </Dialog>
  )
}

/** What is asked of the backend: only what was said - a blank is absent. */
function requestOf(paths: string[], text: string, title: string, kind: string): IntakeRequest {
  return {
    paths,
    ...(text.trim() === '' ? {} : { text }),
    ...(title.trim() === '' ? {} : { title }),
    ...(kind === '' ? {} : { kind }),
  }
}

/** One thing found: what it is called, what it is, and what will happen. */
function FoundRow({ found, kind }: { found: IntakeFound; kind: string | null }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const name = found.name ?? t('intake.pasted')
  const unread = found.standing === null

  return (
    <li className="flex items-start gap-2 border-b border-line px-2.5 py-1.5 last:border-b-0">
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm text-text">{unread ? name : found.title}</p>
        <p className="truncate text-xs text-dim">
          {[
            unread ? null : name,
            kind === null || unread ? null : labelOf(profile.config.work_kinds, kind),
            unread
              ? null
              : t('intake.counts', {
                  versions: found.versions,
                  scores: found.scores,
                  files: found.files,
                }),
          ]
            .filter((part) => part !== null)
            .join(' · ')}
        </p>
        {found.reason !== null && <p className="text-xs text-bad">{sayReason(found.reason)}</p>}
      </div>
      {found.standing !== null && found.reason === null && (
        <Chip variant={found.standing === 'new' ? 'accent' : undefined}>
          {t(`intake.standing.${found.standing}`)}
        </Chip>
      )}
    </li>
  )
}
