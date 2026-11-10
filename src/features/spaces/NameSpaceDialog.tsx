import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { renameSpace } from '@/lib/api/spaces'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { Dialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'

/**
 * The first time the window opens a space kilna named itself - the workspace
 * carried into spaces by v0.109, or the first one of a new machine - it asks
 * what the space is called (ADR 0077). Once: naming it, or putting it off with
 * Cancel for this sitting, and renaming it later from Settings.
 */
export function NameSpaceDialog() {
  const { t } = useTranslation()
  const current = useQuery(queries.currentSpace())
  const [later, setLater] = useState<string | null>(null)
  const [name, setName] = useState<string | null>(null)

  const save = useAppMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) => renameSpace(id, name),
    refresh: [keys.spaces],
    failure: 'spaces.renameFailed',
  })

  const space = current.data?.space
  const asking =
    space !== undefined && space.ask_name && later !== space.id && current.data?.held_by === null
  const value = name ?? space?.name ?? ''
  const submit = () => {
    if (space === undefined || value.trim() === '' || save.isPending) return
    save.mutate({ id: space.id, name: value.trim() })
  }

  return (
    <Dialog
      open={asking}
      onOpenChange={(open) => {
        if (!open && space !== undefined) setLater(space.id)
      }}
      title={t('spaces.nameTitle')}
      description={t('spaces.nameDescription')}
      footer={
        <Button variant="primary" disabled={value.trim() === '' || save.isPending} onClick={submit}>
          {t('spaces.nameSave')}
        </Button>
      }
    >
      <form
        onSubmit={(event) => {
          event.preventDefault()
          submit()
        }}
      >
        <Field label={t('spaces.name')}>
          <Input value={value} onChange={(event) => setName(event.target.value)} maxLength={80} />
        </Field>
      </form>
    </Dialog>
  )
}
