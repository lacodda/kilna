import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { createSpace } from '@/lib/api/spaces'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { Dialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { LookPicker } from '@/features/spaces/LookPicker'
import { SPACE_KINDS, kindLook } from '@/features/spaces/look'
import { useOpenSpace } from '@/features/spaces/useOpenSpace'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Make a space: a name, a kind - Studio, Blog, Novel, Podcast, the shipped
 * profiles as templates - and how it looks. Made and opened at once: a space
 * is made to work in it, not to admire it in a list (ADR 0077).
 *
 * Two spaces of one kind are the point, not an edge: two channels of songs
 * are two registers of repeats, two canons and two calendars that never mix.
 */
export function NewSpaceDialog({ open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const [name, setName] = useState('')
  const [kind, setKind] = useState<string>(SPACE_KINDS[0])
  const [look, setLook] = useState<{ colour: string | null; mark: string | null }>({
    colour: null,
    mark: null,
  })
  const enter = useOpenSpace()

  const make = useAppMutation({
    mutationFn: createSpace,
    refresh: [keys.spaces],
    failure: 'spaces.createFailed',
    onSuccess: (space) => {
      onOpenChange(false)
      setName('')
      setLook({ colour: null, mark: null })
      enter.mutate(space.id)
    },
  })

  const submit = () => {
    if (name.trim() === '' || make.isPending) return
    make.mutate({ name: name.trim(), kind, colour: look.colour, mark: look.mark })
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('spaces.newTitle')}
      description={t('spaces.newDescription')}
      size="lg"
      dirty={look.colour !== null || look.mark !== null}
      footer={
        <Button
          variant="primary"
          disabled={name.trim() === '' || make.isPending || enter.isPending}
          onClick={submit}
        >
          {t('spaces.create')}
        </Button>
      }
    >
      <form
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          submit()
        }}
      >
        <Field label={t('spaces.name')}>
          <Input
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder={t('spaces.namePlaceholder')}
            maxLength={80}
          />
        </Field>
        <FieldGroup label={t('spaces.kind')}>
          <SegmentedControl
            aria-label={t('spaces.kind')}
            value={kind}
            onValueChange={(next) => setKind(next)}
          >
            {SPACE_KINDS.map((entry) => (
              <Segment key={entry} value={entry}>
                {t(`spaces.kinds.${entry}`)}
              </Segment>
            ))}
          </SegmentedControl>
        </FieldGroup>
        <LookPicker
          colour={look.colour ?? kindLook(kind).colour}
          mark={look.mark ?? kindLook(kind).mark}
          onChange={(next) => setLook({ ...look, ...next })}
        />
      </form>
    </Dialog>
  )
}
