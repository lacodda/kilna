import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { applyProposal, createVersion } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { useVocabulary } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/ui/AppDialog'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/ui/AppSelect'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  workId: string
  /** The answer being kept, verbatim. */
  body: string
  /** The role a proposal named, when it named one; the person can still change it. */
  role?: string
  /** A name a proposal gave the version, when it gave one. */
  label?: string
  /** The message carrying the proposal, when the answer is one: inserting
   * then goes through the proposal so the message is marked applied. */
  messageId?: string
}

/**
 * Turns an assistant answer into a version of the work.
 *
 * The person picks the role and decides whether it becomes the current
 * version — off by default, because an answer worth keeping is not yet an
 * answer worth standing behind. The application writes; the assistant never
 * touches the database itself.
 */
export function InsertVersionDialog({
  open,
  onOpenChange,
  workId,
  body,
  role: proposedRole,
  label: proposedLabel,
  messageId,
}: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const navigate = useNavigate()

  const roles = useVocabulary(workId).version_roles
  const [role, setRole] = useState(
    proposedRole !== undefined && roles.some((r) => r.key === proposedRole)
      ? proposedRole
      : (roles[0]?.key ?? ''),
  )
  const [label, setLabel] = useState(proposedLabel ?? '')
  const [makeCurrent, setMakeCurrent] = useState(false)

  const insert = useMutation({
    mutationFn: async (): Promise<string> => {
      const trimmed = label.trim() === '' ? null : label.trim()
      // A proposal is applied as a proposal — the same write, and the
      // message is marked; a plain answer is inserted as a hand would.
      if (messageId !== undefined) {
        const applied = await applyProposal(messageId, {
          role,
          label: trimmed ?? undefined,
          make_current: makeCurrent,
        })
        return applied.versions?.[0] ?? ''
      }
      const version = await createVersion(workId, {
        role,
        body,
        label: trimmed,
        make_current: makeCurrent,
      })
      return version.id
    },
    onSuccess: (versionId) => {
      // The same set a hand-written version disturbs.
      for (const key of [
        keys.journal,
        keys.versions(workId),
        keys.work(workId),
        keys.works,
        keys.transcripts,
      ]) {
        void client.invalidateQueries({ queryKey: key })
      }
      say.ok(t('assistant.inserted'))
      onOpenChange(false)
      // The new version shows itself rather than leaving a toast to vouch for
      // it: the Versions tab opens on the very draft that was just kept.
      void navigate(`/works/${workId}/versions?version=${versionId}`)
    },
    onError: (cause) => {
      say.failedTo(t('toast.versionSaveFailed'), cause)
    },
  })

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('assistant.insertTitle')}
      description={t('assistant.insertBody')}
      footer={
        <Button
          variant="primary"
          disabled={role === '' || insert.isPending}
          onClick={() => {
            insert.mutate()
          }}
        >
          {t('assistant.insert')}
        </Button>
      }
    >
      <div className="flex flex-col gap-3">
        <Select
          value={role}
          onChange={setRole}
          options={roles.map((kind) => ({ value: kind.key, label: kind.label }))}
          aria-label={t('assistant.insertRole')}
        />
        <Input
          value={label}
          onChange={(event) => {
            setLabel(event.target.value)
          }}
          placeholder={t('versions.labelPlaceholder')}
          aria-label={t('versions.labelPlaceholder')}
        />
        <label className="flex cursor-pointer items-center gap-2 text-xs text-dim">
          <input
            type="checkbox"
            className="size-4 accent-[var(--accent)]"
            checked={makeCurrent}
            onChange={(event) => {
              setMakeCurrent(event.target.checked)
            }}
          />
          {t('versions.makeCurrentOnSave')}
        </label>
      </div>
    </Dialog>
  )
}
