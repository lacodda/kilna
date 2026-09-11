import { useTranslation } from 'react-i18next'
import type { Applied, WorkProposal } from '@/lib/api'
import { keys } from '@/lib/query'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { useProfile, vocabularyOf } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { AppliedMark } from '@/components/assistant/AppliedMark'

interface Props {
  /** The work the chat is on; absent when the package proposes a new one. */
  workId?: string
  messageId: string
  proposal: WorkProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
}

/**
 * A whole work, or a package for one, next to the one button that applies
 * all of it.
 *
 * The message above holds everything the package would write, rendered so
 * it can be read first; this is the summary and the decision. A new work is
 * created with its fields, its versions, its score and its notes in one
 * click, and the mark then links to it — the chat is on nothing, and the
 * work is what was made.
 */
export function ProposedWork({ workId, messageId, proposal, applied }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const fresh = workId === undefined
  const kind = proposal.work_kind ?? ''
  const vocabulary = vocabularyOf(profile.config, kind)
  const kindLabel = profile.config.work_kinds.find((k) => k.key === kind)?.label ?? kind

  const apply = useApplyProposal({
    messageId,
    message: t(fresh ? 'assistant.workCreated' : 'assistant.packageApplied'),
    refresh: fresh
      ? [keys.works, keys.catalogue, keys.notes, keys.scores, keys.allChats]
      : [keys.works, keys.work(workId), keys.versions(workId), keys.scores, keys.notes, keys.catalogue],
  })

  const versions = proposal.versions ?? []
  const fields = Object.keys(proposal.fields ?? {})
  const notes = proposal.notes ?? []
  const parts: string[] = []
  if (versions.length > 0) {
    const roles = versions
      .map((v) => vocabulary.version_roles.find((r) => r.key === v.role)?.label ?? v.role)
      .join(', ')
    parts.push(t('assistant.packageVersions', { count: versions.length, roles }))
  }
  if (fields.length > 0) {
    const labels = fields
      .map((key) => profile.config.work_meta_fields.find((f) => f.key === key)?.label ?? key)
      .join(', ')
    parts.push(t('assistant.packageFields', { fields: labels }))
  }
  if (proposal.score !== undefined) {
    parts.push(t('assistant.packageScore', { count: Object.keys(proposal.score.axes).length }))
  }
  if (notes.length > 0) parts.push(t('assistant.packageNotes', { count: notes.length }))

  const unknownFields = proposal.unknown_fields ?? []
  const unknownAxes = proposal.score?.unknown ?? []

  return (
    <div className="mx-3 flex flex-col gap-1.5 rounded-xl border border-line bg-soft px-3 py-2 text-sm">
      <p className="text-xs font-semibold text-dim">
        {fresh ? t('assistant.proposedWork') : t('assistant.proposedPackage')}
        {fresh && proposal.title !== undefined && (
          <>
            {' · '}
            <b className="text-text">{proposal.title}</b>
            {kindLabel !== '' && <span className="font-normal"> · {kindLabel}</span>}
          </>
        )}
      </p>
      {parts.length > 0 && <p className="text-xs text-dim">{parts.join(' · ')}</p>}
      {unknownFields.length > 0 && (
        <p className="text-xs text-warn">
          {t('assistant.packageUnknownFields', { fields: unknownFields.join(', ') })}
        </p>
      )}
      {unknownAxes.length > 0 && (
        <p className="text-xs text-warn">{t('assistant.scoreUnknown', { axes: unknownAxes.join(', ') })}</p>
      )}
      <div className="flex justify-end">
        {applied !== null ? (
          <AppliedMark
            applied={applied}
            label={t(fresh ? 'assistant.workCreatedMark' : 'assistant.packageAppliedMark')}
          />
        ) : (
          <Button
            size="sm"
            variant="primary"
            disabled={apply.isPending}
            onClick={() => {
              apply.mutate(undefined)
            }}
          >
            {t(fresh ? 'assistant.createWork' : 'assistant.applyPackage')}
          </Button>
        )}
      </div>
    </div>
  )
}
