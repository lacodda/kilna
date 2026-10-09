import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { createStyleBrick } from '@/lib/api/styles'
import { harvestTrialPhrase } from '@/lib/api/trials'
import type { Composition } from '@/lib/api/types'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { withExplanation } from '@/lib/styleDraft'
import { styleIconOf } from '@/lib/styleIcon'
import { say as sayLabel, styleTypesOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Dialog } from '@/components/AppDialog'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  composition: Composition
  /** The phrase as the text writes it. */
  phrase: string
  /** The kept trial of an experiment the phrase is cut from (v0.95): the
   *  brick remembers it, and the trial shows where it went. */
  trialId?: string
}

/**
 * A phrase a text says, kept in the dictionary by hand (v0.94): its type -
 * one of the composition's - the phrase as written, what it means in the
 * window's language and when to take it. A brick of the owner's own, ready
 * at once: a phrase is its description.
 */
export function AddPhraseDialog({ open, onOpenChange, composition, phrase, trialId }: Props) {
  const { t } = useTranslation()
  const types = styleTypesOf(useProfile().config).filter((type) =>
    composition.parts.some((part) => part.type === type.key),
  )
  const [typeKey, setTypeKey] = useState<string>(types[0]?.key ?? '')
  const [family, setFamily] = useState<string>('')
  const [words, setWords] = useState(phrase)
  const [meaning, setMeaning] = useState('')
  const [when, setWhen] = useState('')
  const type = types.find((one) => one.key === typeKey)
  const families = type?.families ?? []

  const add = useAppMutation({
    mutationFn: () => {
      const brick = {
        type_key: typeKey,
        name: words.trim(),
        description: words.trim(),
        family: family === '' ? undefined : family,
        when_to_use: when.trim() === '' ? undefined : when.trim(),
        explanation: withExplanation(null, meaning) ?? undefined,
      }
      return trialId === undefined ? createStyleBrick(brick) : harvestTrialPhrase(trialId, brick)
    },
    failure: 'phrases.addFailed',
    // A phrase cut from a trial is that trial's harvest, shown on its board.
    refresh: [...refresh.style, ['works', 'trials']],
    onSuccess: (brick) => {
      say.ok(t('phrases.added', { phrase: brick.name }))
      onOpenChange(false)
    },
  })

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('phrases.addTitle')}
      description={t('phrases.addBody')}
      size="lg"
      footer={
        <Button
          variant="primary"
          disabled={words.trim() === '' || typeKey === '' || add.isPending}
          onClick={() => add.mutate()}
        >
          {t('phrases.add')}
        </Button>
      }
    >
      <div className="flex flex-col gap-4">
        <Field label={t('styles.phrase')} help={t('styles.phraseHint')}>
          <Input
            value={words}
            onChange={(event) => setWords(event.target.value)}
            className="font-mono"
          />
        </Field>
        <FieldGroup
          label={t('styles.type')}
          help={type?.hint == null ? undefined : sayLabel(type.hint)}
        >
          <ChipGroup
            value={typeKey === '' ? [] : [typeKey]}
            onValueChange={([next]) => {
              if (next === undefined) return
              setTypeKey(next)
              setFamily('')
            }}
          >
            {types.map((one) => {
              const Icon = styleIconOf(one)
              return (
                <Chip key={one.key} value={one.key}>
                  <Icon aria-hidden className="size-3.5" />
                  {sayLabel(one.label)}
                </Chip>
              )
            })}
          </ChipGroup>
        </FieldGroup>
        {families.length > 0 && (
          <FieldGroup label={t('styles.family')}>
            <ChipGroup
              value={family === '' ? [] : [family]}
              onValueChange={([next]) => setFamily(next ?? '')}
            >
              {families.map((one) => (
                <Chip key={one.key} value={one.key}>
                  {sayLabel(one.label)}
                </Chip>
              ))}
            </ChipGroup>
          </FieldGroup>
        )}
        <Field label={t('styles.explanation')} help={t('styles.explanationHint')}>
          <Textarea
            autoResize
            rows={2}
            value={meaning}
            onChange={(event) => setMeaning(event.target.value)}
          />
        </Field>
        <Field label={t('styles.when')} help={t('phrases.whenHint')}>
          <Input value={when} onChange={(event) => setWhen(event.target.value)} />
        </Field>
      </div>
    </Dialog>
  )
}
