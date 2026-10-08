import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Collection } from '@/lib/api/types'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Dialog } from '@/components/AppDialog'
import { Select } from '@/components/AppSelect'
import { useCollectionGestures } from './useCollectionGestures'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** The works it is made for - the ticked rows, the row carried onto it,
   *  the open work - put in it as soon as it exists. */
  workIds?: readonly string[]
  /** Called with what was made, once it holds them. */
  onMade?: (collection: Collection) => void
}

/**
 * The one door through which a collection is made: a name and what kind of
 * thing it is. The goal, the day and the order are the collection's page's
 * business - asking for them here would be asking for answers nobody has yet,
 * at the moment of starting.
 */
export function NewCollectionDialog({ open, onOpenChange, workIds = [], onMade }: Props) {
  return open ? <Form workIds={workIds} onOpenChange={onOpenChange} onMade={onMade} /> : null
}

function Form({
  workIds,
  onOpenChange,
  onMade,
}: Required<Pick<Props, 'workIds' | 'onOpenChange'>> & Pick<Props, 'onMade'>) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const kinds = config.collection_kinds
  const [title, setTitle] = useState('')
  const [kind, setKind] = useState(kinds[0]?.key ?? '')
  const { make } = useCollectionGestures()

  const submit = () => {
    const name = title.trim()
    if (name === '' || kind === '' || make.isPending) return
    make.mutate(
      { title: name, kind, workIds: [...workIds] },
      {
        onSuccess: ({ made }) => {
          onOpenChange(false)
          onMade?.(made)
        },
      },
    )
  }

  return (
    <Dialog
      open
      onOpenChange={onOpenChange}
      title={t('collections.newTitle')}
      description={
        workIds.length > 0 ? t('collections.newFor', { count: workIds.length }) : undefined
      }
      footer={
        <Button
          type="submit"
          form="new-collection"
          variant="primary"
          disabled={title.trim() === '' || kind === '' || make.isPending}
        >
          {t('collections.make')}
        </Button>
      }
    >
      <form
        id="new-collection"
        className="flex flex-col gap-3"
        onSubmit={(event) => {
          event.preventDefault()
          submit()
        }}
      >
        <Field label={t('collections.name')}>
          <Input
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder={t('collections.namePlaceholder')}
          />
        </Field>
        {/* A craft that names no kinds of collection has nothing to make one
            of: said rather than offered as an empty choice. */}
        {kinds.length === 0 ? (
          <p className="text-sm text-bad">{t('collections.noKinds')}</p>
        ) : (
          kinds.length > 1 && (
            <FieldGroup label={t('collections.kind')}>
              <Select
                aria-label={t('collections.kind')}
                value={kind}
                onChange={setKind}
                options={kinds.map((entry) => ({ value: entry.key, label: sayLabel(entry.label) }))}
              />
            </FieldGroup>
          )
        )}
      </form>
    </Dialog>
  )
}
