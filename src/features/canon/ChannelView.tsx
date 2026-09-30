import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { deleteNote, updateNote } from '@/lib/api/notes'
import type { CardView } from '@/lib/api/types'
import { cardKindOf, type LensChoice } from '@/lib/canon'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceDeleted } from '@/lib/trash'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Input } from '@/components/ui/input'
import { RowMenu } from '@/components/RowMenu'
import { Scroll } from '@/components/frame'
import { CardAvatar } from '@/features/canon/CardAvatar'
import { PromptBox } from '@/features/canon/PromptBox'
import { SectionBlock } from '@/features/canon/SectionBlock'
import { cn } from '@/lib/utils'

interface Props {
  view: CardView
  lens: LensChoice
  onOpen: (id: string) => void
  onGone: () => void
}

/**
 * The channel: the root card of the canon, laid out as a board of typed
 * panels rather than a column of statements - each section has its own reader
 * downstream (the switches of the cover constructor, the choice of the mark,
 * the slots of a frame's lettering, the text a release goes out under) and
 * says so under its name. Its description for a generator is the mark's.
 */
export function ChannelView({ view, lens, onOpen, onGone }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const { config } = useProfile()
  const card = view.card
  const kind = cardKindOf(config, card.kind)
  const [title, setTitle] = useState(card.title ?? '')

  const rename = useAppMutation({
    mutationFn: (next: string) => updateNote(card.id, { title: next }),
    failure: 'toast.cardSaveFailed',
    refresh: refresh.canon,
  })
  const remove = useAppMutation({
    mutationFn: () => deleteNote(card.id),
    failure: 'toast.cardSaveFailed',
    onSuccess: (deletionId) => {
      announceDeleted({
        client,
        deletionId,
        message: t('canon.cardDeleted', { title: card.title ?? '' }),
        refresh: refresh.canon,
      })
      onGone()
    },
  })

  const sections = kind?.sections ?? []
  // The mark's variants are read as a row across the board; the rest pair up.
  const wide = (shape: string | undefined) => shape === 'marks'

  return (
    <section className="flex min-h-0 min-w-0 flex-1 flex-col gap-2.5">
      <header className="flex shrink-0 items-center gap-2.5 rounded-lg border border-line bg-raise px-3 py-2">
        <CardAvatar
          title={card.title}
          portrait={
            [...view.pictures].reverse().find((one) => one.kind === 'portrait')?.path ?? null
          }
          size="lg"
        />
        <div className="flex min-w-0 flex-1 flex-col">
          <Input
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            onBlur={() => {
              const next = title.trim()
              if (next !== '' && next !== (card.title ?? '')) rename.mutate(next)
              else setTitle(card.title ?? '')
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter') event.currentTarget.blur()
            }}
            aria-label={t('canon.name')}
            className="border-transparent bg-transparent px-1 text-lg font-semibold hover:border-line focus:border-line"
          />
          <span className="px-1 text-xs text-faint">
            {kind === undefined ? card.kind : sayLabel(kind.label)} · {t('canon.channelHint')}
          </span>
        </div>
        <RowMenu
          label={t('canon.cardActions')}
          actions={[
            {
              key: 'delete',
              label: t('canon.deleteCard'),
              danger: true,
              onSelect: () => remove.mutate(),
            },
          ]}
        />
      </header>
      <Scroll label={card.title ?? t('canon.untitled')} contentClassName="p-0.5">
        <div className="grid grid-cols-2 items-start gap-2.5 max-[1100px]:grid-cols-1">
          {sections.map((section) => (
            <div
              key={section.key}
              className={cn(wide(section.shape) && 'col-span-2 max-[1100px]:col-span-1')}
            >
              <SectionBlock view={view} section={section} lens={lens} onOpen={onOpen} panel />
              {section.shape === 'marks' && <PromptBox view={view} describing={false} />}
            </div>
          ))}
        </div>
      </Scroll>
    </section>
  )
}
