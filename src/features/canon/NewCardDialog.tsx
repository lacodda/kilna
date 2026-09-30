import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link2 } from 'lucide-react'
import { createCard } from '@/lib/api/canon'
import type { CardSummary } from '@/lib/api/types'
import { cardKindOf, cardKindsOf } from '@/lib/canon'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Dialog } from '@/components/AppDialog'
import { Select } from '@/components/AppSelect'
import { PickWorkDialog } from '@/components/PickWorkDialog'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Every card, to know whether the root is taken. */
  cards: CardSummary[]
  onCreated: (id: string) => void
}

/**
 * A new card: its kind, its name, and - for a hero who lives in one song -
 * the work it lives at. The root kind is offered only while the canon has no
 * root: there is one channel.
 */
export function NewCardDialog({ open, onOpenChange, cards, onCreated }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const kinds = cardKindsOf(config).filter((key) => {
    const kind = cardKindOf(config, key)
    return kind?.root !== true || !cards.some((card) => card.kind === key)
  })
  const [kind, setKind] = useState(
    kinds.find((key) => cardKindOf(config, key)?.root !== true) ?? kinds[0] ?? '',
  )
  const [title, setTitle] = useState('')
  const [work, setWork] = useState<{ id: string; title: string } | null>(null)
  const [picking, setPicking] = useState(false)

  const create = useAppMutation({
    mutationFn: () =>
      createCard({ body: '', kind, title: title.trim(), work_id: work?.id ?? undefined }),
    failure: 'toast.cardSaveFailed',
    refresh: refresh.canon,
    onSuccess: (card) => onCreated(card.id),
  })

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('canon.newCard')}
      footer={
        <Button
          variant="primary"
          disabled={kind === '' || title.trim() === '' || create.isPending}
          onClick={() => create.mutate()}
        >
          {t('canon.create')}
        </Button>
      }
    >
      <Field label={t('canon.kind')}>
        <Select
          value={kind}
          onChange={setKind}
          options={kinds.map((key) => ({
            value: key,
            label: sayLabel(cardKindOf(config, key)?.label),
          }))}
        />
      </Field>
      <Field label={t('canon.name')}>
        <Input
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter' && title.trim() !== '' && kind !== '') create.mutate()
          }}
        />
      </Field>
      <div className="flex flex-col gap-1">
        <span className="text-xs text-dim">{t('canon.livesAt')}</span>
        {work === null ? (
          <Button size="sm" variant="ghost" className="self-start" onClick={() => setPicking(true)}>
            <Link2 aria-hidden />
            {t('canon.pickWork')}
          </Button>
        ) : (
          <Chip
            onRemove={() => setWork(null)}
            removeLabel={t('canon.dropWork')}
            className="self-start"
          >
            {work.title}
          </Chip>
        )}
        <span className="text-2xs text-faint">{t('canon.livesAtHint')}</span>
      </div>
      {picking && (
        <PickWorkDialog
          open
          onOpenChange={setPicking}
          title={t('canon.pickWork')}
          onPick={(picked) => {
            setPicking(false)
            setWork({ id: picked.work_id, title: picked.title })
          }}
        />
      )}
    </Dialog>
  )
}
