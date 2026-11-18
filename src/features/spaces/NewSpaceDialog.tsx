import { say } from '@/lib/toast'
import { Checkbox } from '@/components/ui/checkbox'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { createSpace, seedDemo } from '@/lib/api/spaces'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { Dialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { LookPicker } from '@/features/spaces/LookPicker'
import { SPACE_KINDS, kindLook } from '@/features/spaces/look'
import { useOpenSpace } from '@/features/spaces/useOpenSpace'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Make a space: a name, what it is made of - Studio, Blog, Novel, Podcast or
 * the empty kind, the shipped profiles as templates, or a type kept on this
 * machine (v0.112, ADR 0079) - and how it looks. Made and opened at once: a space
 * is made to work in it, not to admire it in a list (ADR 0077).
 *
 * Two spaces of one kind are the point, not an edge: two channels of songs
 * are two registers of repeats, two canons and two calendars that never mix.
 */
export function NewSpaceDialog({ open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const [name, setName] = useState('')
  // What it is made of: a shipped kind by its key, or a type as `type:<id>`.
  const [madeOf, setMadeOf] = useState<string>(SPACE_KINDS[0])
  const types = useQuery(queries.spaceTypes()).data ?? []
  const type = types.find((entry) => `type:${entry.id}` === madeOf)
  // A space made of a type is of the kind the type grew from.
  const kind = type?.kind ?? (madeOf.startsWith('type:') ? SPACE_KINDS[0] : madeOf)
  const [look, setLook] = useState<{ colour: string | null; mark: string | null }>({
    colour: null,
    mark: null,
  })
  const enter = useOpenSpace()
  const [demo, setDemo] = useState(false)
  const client = useQueryClient()

  const make = useAppMutation({
    mutationFn: createSpace,
    refresh: [keys.spaces],
    failure: 'spaces.createFailed',
    onSuccess: (space) => {
      onOpenChange(false)
      setName('')
      setLook({ colour: null, mark: null })
      enter.mutate(space.id, {
        onSuccess: () => {
          if (!demo) return
          setDemo(false)
          seedDemo()
            .then(async (report) => {
              await client.resetQueries()
              say.ok(t('welcome.demoAdded', { count: report.works }))
            })
            .catch((cause: unknown) => say.failed(cause))
        },
      })
    },
  })

  const submit = () => {
    if (name.trim() === '' || make.isPending) return
    make.mutate({
      name: name.trim(),
      kind,
      colour: look.colour,
      mark: look.mark,
      ...(type === undefined ? {} : { type_id: type.id }),
    })
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
        <FieldGroup
          label={t('spaces.kind')}
          help={
            type === undefined
              ? t(`spaces.kindHint.${kind}`)
              : (type.description ?? t('spaces.typeHint', { kind: t(`spaces.kinds.${type.kind}`) }))
          }
        >
          {/* Chips rather than a segment since v0.112: the shipped kinds and
              the person's own types are one choice, and a row of them wraps. */}
          <ChipGroup
            aria-label={t('spaces.kind')}
            value={[madeOf]}
            onValueChange={(next) => {
              const picked = next.find((entry) => entry !== madeOf)
              if (picked !== undefined) setMadeOf(picked)
            }}
          >
            {SPACE_KINDS.map((entry) => (
              <Chip key={entry} value={entry}>
                {t(`spaces.kinds.${entry}`)}
              </Chip>
            ))}
            {types.map((entry) => (
              <Chip key={entry.id} value={`type:${entry.id}`}>
                {entry.name}
              </Chip>
            ))}
          </ChipGroup>
        </FieldGroup>
        <LookPicker
          colour={look.colour ?? kindLook(kind).colour}
          mark={look.mark ?? kindLook(kind).mark}
          onChange={(next) => setLook({ ...look, ...next })}
        />
        <Checkbox checked={demo} onCheckedChange={(next) => setDemo(next === true)}>
          {t('welcome.space.demo')}
        </Checkbox>
      </form>
    </Dialog>
  )
}
