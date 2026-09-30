import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { relateCards, updateRelation } from '@/lib/api/canon'
import type { Layer, Note, Relation } from '@/lib/api/types'
import { LAYERS } from '@/lib/canon'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Dialog } from '@/components/AppDialog'
import { Select } from '@/components/AppSelect'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** The card the relation is drawn from. */
  card: Note
  /** The kinds of card the section gathers; any when empty. */
  kinds: string[]
  /** The relation to change, when it is drawn already: the other card stays. */
  relation?: Relation
}

/**
 * Drawing a relation between two cards: which card, what kind of relation,
 * and the words each side uses for the other - the neighbour of one is the
 * neighbour and first listener of the other - and the layer it may be told in.
 * Given a relation, the same dialog changes it; the other card stays.
 */
export function RelationDialog({ open, onOpenChange, card, kinds, relation }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const cards = useQuery({ ...queries.cardsMatching({}), enabled: open && !relation })
  const link = relation?.link
  // Whether this card is the end the relation was drawn from: the row keeps
  // its words the way it was first drawn, and the dialog reads them from here.
  const forward = link === undefined || link.from_id === card.id
  const [other, setOther] = useState(relation?.other_id ?? '')
  const [kind, setKind] = useState(link?.kind ?? '')
  const [label, setLabel] = useState((forward ? link?.label : link?.back_label) ?? '')
  const [back, setBack] = useState((forward ? link?.back_label : link?.label) ?? '')
  const [layer, setLayer] = useState<Layer>(link?.layer ?? 'public')
  const relationKinds = config.relation_kinds ?? []

  const candidates = (cards.data ?? []).filter(
    (one) => one.id !== card.id && (kinds.length === 0 || kinds.includes(one.kind)),
  )
  const otherTitle =
    relation?.other_title ?? candidates.find((one) => one.id === other)?.title ?? '…'
  const words = (text: string) => (text.trim() === '' ? null : text.trim())

  const relate = useAppMutation({
    mutationFn: () =>
      link === undefined
        ? relateCards({
            from_id: card.id,
            to_id: other,
            kind: kind === '' ? undefined : kind,
            label: words(label) ?? undefined,
            back_label: words(back) ?? undefined,
            layer,
          })
        : updateRelation(link.id, {
            kind: kind === '' ? null : kind,
            label: forward ? words(label) : words(back),
            back_label: forward ? words(back) : words(label),
            layer,
          }),
    failure: 'canon.relateFailed',
    refresh: refresh.canon,
    onSuccess: () => onOpenChange(false),
  })

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={relation ? t('canon.editRelation') : t('canon.addRelation')}
      dirty={other !== ''}
      footer={
        <Button
          variant="primary"
          disabled={other === '' || relate.isPending}
          onClick={() => relate.mutate()}
        >
          {relation ? t('canon.saveFact') : t('canon.relate')}
        </Button>
      }
    >
      {relation ? (
        <p className="text-sm font-semibold">{otherTitle}</p>
      ) : (
        <Field label={t('canon.relatedCard')}>
          <Select
            value={other}
            onChange={setOther}
            placeholder={t('canon.pickCard')}
            options={candidates.map((one) => ({
              value: one.id,
              label: one.title ?? t('canon.untitled'),
            }))}
          />
        </Field>
      )}
      {relationKinds.length > 0 && (
        <Field label={t('canon.relationKind')}>
          <Select
            value={kind}
            onChange={setKind}
            placeholder={t('canon.anyRelationKind')}
            options={relationKinds.map((one) => ({
              value: one.key,
              label: labelOf(relationKinds, one.key),
            }))}
          />
        </Field>
      )}
      <Field label={t('canon.relationLabel', { from: card.title ?? '', to: otherTitle })}>
        <Input value={label} onChange={(event) => setLabel(event.target.value)} />
      </Field>
      <Field label={t('canon.relationBackLabel', { from: card.title ?? '', to: otherTitle })}>
        <Input value={back} onChange={(event) => setBack(event.target.value)} />
      </Field>
      <Field label={t('canon.layerLabel')}>
        <Select
          value={layer}
          onChange={(next) => setLayer(next as Layer)}
          options={LAYERS.map((one) => ({ value: one, label: t(`canon.layer.long.${one}`) }))}
        />
      </Field>
    </Dialog>
  )
}
