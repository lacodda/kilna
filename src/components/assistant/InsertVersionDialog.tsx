import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { createVersion } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Dialog } from '@/components/ui/Dialog'
import { Input } from '@/components/ui/Input'
import { Select } from '@/components/ui/Select'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  workId: string
  /** The answer being kept, verbatim. */
  body: string
}

/**
 * Turns an assistant answer into a version of the work.
 *
 * The person picks the role and decides whether it becomes the current
 * version — off by default, because an answer worth keeping is not yet an
 * answer worth standing behind. The application writes; the assistant never
 * touches the database itself.
 */
export function InsertVersionDialog({ open, onOpenChange, workId, body }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()

  const roles = profile.config.version_roles
  const [role, setRole] = useState(roles[0]?.key ?? '')
  const [label, setLabel] = useState('')
  const [makeCurrent, setMakeCurrent] = useState(false)

  const insert = useMutation({
    mutationFn: () =>
      createVersion(workId, {
        role,
        body,
        label: label.trim() === '' ? null : label.trim(),
        make_current: makeCurrent,
      }),
    onSuccess: () => {
      // The same set a hand-written version disturbs.
      for (const key of [keys.journal, keys.versions(workId), keys.work(workId), keys.works]) {
        void client.invalidateQueries({ queryKey: key })
      }
      say.ok(t('assistant.inserted'))
      onOpenChange(false)
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
