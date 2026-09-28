import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import type { Work } from '@/lib/api/types'
import { unpinStatus } from '@/lib/api/works'
import { fieldsOf } from '@/lib/overview'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { labelOf, say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { SaveState, useSaveStatus } from '@/components/ui/save-state'
import { Select } from '@/components/AppSelect'
import { MetaInput } from '@/features/work/tabs/overview/MetaInput'
import { useLook, Widget } from '@/features/work/tabs/overview/Widget'
import { useWorkEdit } from '@/features/work/tabs/overview/useWorkEdit'

/**
 * The work's own fields, edited where they are read: the widget catalogue's
 * `fields` kind - the tempo, the key, the length - each an InlineField.
 *
 * This is what the overview was until v0.82, a form, folded into one widget
 * of the board. The title is renamed in the card's header. The status and the
 * kind stand first here because the header only reads them, and a status
 * chosen by hand - pinned, so the automation steps over the work - and the
 * way back from that were only ever set on this tab.
 *
 * The paragraphs are the hook widget's (`fieldsOf`): a premise in a grid cell
 * was a column of two-word lines.
 */
export function FieldsWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const { presentation } = useLook()
  const { patch, setField } = useWorkEdit(work)
  const saveStatus = useSaveStatus(patch.isPending, patch.isError)

  const { short } = fieldsOf(profile.config.work_meta_fields)
  const line = presentation === 'line'

  return (
    <Widget
      caption={t('overview.fields', { kind: labelOf(profile.config.work_kinds, work.kind) })}
      // A word that appears for a second while saving, at the caption's end
      // where it takes no room from the fields.
      aside={
        <SaveState
          savingLabel={t('save.saving')}
          savedLabel={t('save.saved')}
          status={saveStatus}
        />
      }
    >
      <div
        className={cn(
          line
            ? 'flex flex-wrap items-end gap-x-4.5 gap-y-1.5'
            : // Columns of a shared width rather than a wrapping row: fields
              // of different widths wrap into a ragged shape the moment the
              // widget narrows, with holes beside the short ones.
              'grid grid-cols-[repeat(auto-fill,minmax(6.5rem,1fr))] items-start gap-x-3 gap-y-2',
        )}
      >
        <StatusField work={work} onChange={(status) => patch.mutate({ status })} />
        <FieldGroup label={t('work.kind')}>
          <Select
            aria-label={t('work.kind')}
            className="h-control-sm w-full text-xs"
            value={work.kind}
            onChange={(kind) => patch.mutate({ kind })}
            options={profile.config.work_kinds.map((kind) => ({
              value: kind.key,
              label: sayLabel(kind.label),
            }))}
          />
        </FieldGroup>

        {short.map((field) => (
          <MetaInput
            key={field.key}
            field={field}
            value={work.meta[field.key]}
            onChange={(value) => setField(field.key, value)}
            inline={line}
          />
        ))}
      </div>
    </Widget>
  )
}

/**
 * Where the work stands, and whose word that is. Picking a status pins it -
 * the automation then steps over this work entirely - and the only way back
 * is to say so, under the box.
 */
function StatusField({ work, onChange }: { work: Work; onChange: (status: string) => void }) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const profile = useProfile()
  const vocabulary = vocabularyOf(profile.config, work.kind)
  const pinned = work.status_pinned_at != null

  const unpin = useAppMutation({
    mutationFn: () => unpinStatus(work.id),
    failure: 'toast.workSaveFailed',
    refresh: refresh.work,
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
    },
  })

  return (
    // The sentence that explains the status rides on the hover: a cell of the
    // grid has room for the box and one short line, and the line is the way
    // back when there is one.
    <div
      className="flex min-w-0 flex-col gap-0.5"
      title={pinned ? t('work.statusPinned') : t('work.statusDerived')}
    >
      <FieldGroup label={t('work.status')}>
        <Select
          aria-label={t('work.status')}
          className="h-control-sm w-full text-xs"
          value={work.status}
          onChange={onChange}
          options={vocabulary.statuses.map((s) => ({ value: s.key, label: sayLabel(s.label) }))}
        />
      </FieldGroup>
      {pinned && (
        <span className="text-xs">
          <Button
            variant="link"
            onClick={() => unpin.mutate()}
            disabled={unpin.isPending}
            title={t('work.unpinStatusHint')}
          >
            {t('work.unpinStatus')}
          </Button>
        </span>
      )}
    </div>
  )
}
