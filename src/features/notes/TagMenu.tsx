import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { retagNotes } from '@/lib/api/notes'
import { announceEdited } from '@/lib/edited'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Dialog } from '@/components/AppDialog'
import { RowMenu } from '@/components/RowMenu'

interface Props {
  /** The tag the screen narrows to. */
  tag: string
  /** Where the screen narrows to once the tag is renamed or gone. */
  onTag: (tag: string) => void
}

/**
 * What can be done to a tag itself, rather than to one note (ADR 0064):
 * renamed on every note that carries it, or taken off them all - one gesture,
 * one undo. A tag is a word many notes share; changing it note by note would
 * be a thousand gestures for one decision.
 */
export function TagMenu({ tag, onTag }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [renaming, setRenaming] = useState(false)
  const [name, setName] = useState(tag)

  const retag = useAppMutation({
    mutationFn: (to: string | null) => retagNotes(tag, to),
    failure: 'notes.retagFailed',
    onSuccess: (count, to) => {
      announceEdited({
        client,
        message:
          to === null
            ? t('notes.untagged', { tag, count })
            : t('notes.retagged', { tag, to, count }),
        refresh: refresh.note,
      })
      setRenaming(false)
      onTag(to ?? '')
    },
  })

  const next = name.trim()
  return (
    <>
      <RowMenu
        label={t('notes.tagMenu')}
        actions={[
          {
            key: 'rename',
            label: t('notes.retag'),
            onSelect: () => {
              setName(tag)
              setRenaming(true)
            },
          },
          {
            key: 'untag',
            label: t('notes.untag'),
            onSelect: () => retag.mutate(null),
            danger: true,
          },
        ]}
      />
      <Dialog
        open={renaming}
        onOpenChange={setRenaming}
        title={t('notes.retagTitle', { tag })}
        footer={
          <Button
            type="submit"
            form="retag"
            variant="primary"
            disabled={next === '' || next === tag || retag.isPending}
          >
            {t('notes.retag')}
          </Button>
        }
      >
        <form
          id="retag"
          onSubmit={(event) => {
            event.preventDefault()
            if (next !== '' && next !== tag) retag.mutate(next)
          }}
        >
          <Input
            autoFocus
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder={t('notes.retagPlaceholder')}
            aria-label={t('notes.retagPlaceholder')}
          />
        </form>
      </Dialog>
    </>
  )
}
