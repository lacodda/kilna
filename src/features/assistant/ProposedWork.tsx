import { useTranslation } from 'react-i18next'
import type { Applied, WorkProposal } from '@/lib/api/types'
import { fieldText } from '@/lib/fieldValue'
import { keys } from '@/lib/query/keys'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { say, useProfile, vocabularyOf } from '@/lib/useProfile'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  /** The work the chat is on; absent when the package proposes a new one. */
  workId?: string
  messageId: string
  proposal: WorkProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
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
export function ProposedWork({ workId, messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const fresh = workId === undefined
  const kind = proposal.work_kind ?? ''
  const vocabulary = vocabularyOf(profile.config, kind)
  const kindLabelRaw = profile.config.work_kinds.find((k) => k.key === kind)?.label
  const kindLabel = kindLabelRaw === undefined ? kind : say(kindLabelRaw)

  const apply = useApplyProposal({
    messageId,
    message: t(fresh ? 'assistant.workCreated' : 'assistant.packageApplied'),
    refresh: fresh
      ? [keys.works, keys.catalogue, keys.notes, keys.scores, keys.scenes, keys.allChats]
      : [
          keys.works,
          keys.work(workId),
          keys.versions(workId),
          keys.scores,
          keys.notes,
          keys.scenes,
          keys.catalogue,
        ],
  })

  const versions = proposal.versions ?? []
  const fields = Object.keys(proposal.fields ?? {})
  const notes = proposal.notes ?? []
  const parts: string[] = []
  if (versions.length > 0) {
    const roles = versions
      .map((v) => {
        const label = vocabulary.version_roles.find((r) => r.key === v.role)?.label
        return label === undefined ? v.role : say(label)
      })
      .join(', ')
    parts.push(t('assistant.packageVersions', { count: versions.length, roles }))
  }
  if (fields.length > 0) {
    const labels = fields
      .map((key) => {
        const field = profile.config.work_meta_fields.find((f) => f.key === key)
        if (field === undefined) return key
        // A choice says which answer, by its label: "Variant: Slowed" is the
        // whole of what the package would set, where "Variant" alone leaves
        // the one thing worth reading out.
        if (field.type !== 'choice') return say(field.label)
        const value = fieldText(field, proposal.fields?.[key])
        return value === null ? say(field.label) : `${say(field.label)}: ${value}`
      })
      .join(', ')
    parts.push(t('assistant.packageFields', { fields: labels }))
  }
  if (proposal.score !== undefined && proposal.score !== null) {
    parts.push(t('assistant.packageScore', { count: Object.keys(proposal.score.axes).length }))
  }
  if (notes.length > 0) parts.push(t('assistant.packageNotes', { count: notes.length }))
  const scenes = proposal.scenes ?? []
  if (scenes.length > 0) parts.push(t('assistant.packageScenes', { count: scenes.length }))

  const unknownFields = proposal.unknown_fields ?? []
  const unknownAxes = proposal.score?.unknown ?? []
  const warnings = [
    ...(unknownFields.length > 0
      ? [t('assistant.packageUnknownFields', { fields: unknownFields.join(', ') })]
      : []),
    ...(unknownAxes.length > 0
      ? [t('assistant.scoreUnknown', { axes: unknownAxes.join(', ') })]
      : []),
  ]

  return (
    <ProposalCard
      messageId={messageId}
      title={fresh ? t('assistant.proposedWork') : t('assistant.proposedPackage')}
      warnings={warnings}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t(fresh ? 'assistant.createWork' : 'assistant.applyPackage')}
      onApply={() => {
        apply.mutate(undefined)
      }}
      applying={apply.isPending}
      appliedLabel={t(fresh ? 'assistant.workCreatedMark' : 'assistant.packageAppliedMark')}
    >
      {fresh && proposal.title !== undefined && (
        <p>
          <b className="font-semibold text-text">{proposal.title}</b>
          {kindLabel !== '' && <> · {kindLabel}</>}
        </p>
      )}
      {parts.length > 0 && <p>{parts.join(' · ')}</p>}
    </ProposalCard>
  )
}
