import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open as openFile } from '@tauri-apps/plugin-dialog'
import { Eye, ImagePlus } from 'lucide-react'
import { attachAsset, detachAsset } from '@/lib/api/assets'
import { pasteCardPicture, setPictureRole } from '@/lib/api/canon'
import type { CardView } from '@/lib/api/types'
import type { LensChoice } from '@/lib/canon'
import { picturesAmong } from '@/lib/drop'
import { PICTURES } from '@/lib/media'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { RowButton } from '@/components/ui/list-row'
import { Pane } from '@/components/frame'
import { CardAvatar } from '@/features/canon/CardAvatar'
import { CardPicture } from '@/features/canon/CardPicture'
import { CanonProposals } from '@/features/canon/CanonProposals'
import { SeenDialog } from '@/features/canon/SeenDialog'
import { pictureIn, readPicture, type PastedPicture } from '@/features/styles/pastedPicture'
import { useWindowDrop } from '@/features/styles/useWindowDrop'

interface Props {
  view: CardView
  lens: LensChoice
  onOpen: (id: string) => void
}

/**
 * The right of an open card: its pictures, each with its role - the ones a
 * generator is handed beside the description - the cards it stands beside,
 * what the assistant proposes for it, and how the assistant reads it.
 */
export function CardSide({ view, lens, onOpen }: Props) {
  const { t } = useTranslation()
  const zone = useRef<HTMLDivElement>(null)
  const [seeing, setSeeing] = useState(false)
  const card = view.card
  const own = view.pictures.filter((picture) => picture.canon_fact_id === null)

  const attach = useAppMutation({
    mutationFn: async (paths: string[]) => {
      for (const path of paths) await attachAsset(path, { note_id: card.id, kind: 'reference' })
    },
    failure: 'canon.pictureFailed',
    refresh: refresh.canon,
  })
  const paste = useAppMutation({
    mutationFn: ({ bytes, name }: PastedPicture) =>
      pasteCardPicture({ note_id: card.id, kind: 'reference' }, bytes, name),
    failure: 'canon.pictureFailed',
    refresh: refresh.canon,
  })
  const pasteMutate = paste.mutate
  const role = useAppMutation({
    mutationFn: ({ id, next }: { id: string; next: string }) => setPictureRole(id, next),
    failure: 'canon.pictureFailed',
    refresh: refresh.canon,
  })
  const detach = useAppMutation({
    mutationFn: (id: string) => detachAsset(id),
    refresh: refresh.canon,
  })

  const add = (paths: string[]) => {
    const { pictures, others } = picturesAmong(paths)
    if (others > 0) say.warn(t('styles.notPictures', { count: others }))
    if (pictures.length > 0) attach.mutate(pictures)
  }
  const over = useWindowDrop(zone, add)

  // A picture pasted while a card is open is the card's, as a pasted
  // reference is the open style's. Not inside a field: text pasted there is
  // the field's business.
  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      const file = pictureIn(event)
      if (file === undefined) return
      event.preventDefault()
      readPicture(file).then(
        (pasted) => pasteMutate(pasted),
        (cause: unknown) => say.failed(cause),
      )
    }
    window.addEventListener('paste', onPaste)
    return () => window.removeEventListener('paste', onPaste)
  }, [pasteMutate])

  const choose = async () => {
    const chosen = await openFile({
      multiple: true,
      title: t('canon.addPicture'),
      filters: [{ name: t('styles.pictures'), extensions: [...PICTURES] }],
    })
    const paths = chosen === null ? [] : Array.isArray(chosen) ? chosen : [chosen]
    if (paths.length > 0) add(paths.filter((path) => typeof path === 'string'))
  }

  const proposals = useQuery(queries.canonProposals())
  const mine = (proposals.data ?? []).filter((one) => one.cards.includes(card.id))

  return (
    <Pane label={t('canon.side')} bodyClassName="flex flex-col gap-4 p-3">
      <section
        ref={zone}
        className="flex flex-col gap-1.5 rounded-lg"
        data-dropping={over ? '' : undefined}
      >
        <header className="flex items-center gap-2">
          <b className="text-sm font-semibold">{t('canon.pictures')}</b>
          <span className="text-2xs text-faint">{t('canon.picturesHint')}</span>
        </header>
        {own.length > 0 && (
          <div className="grid grid-cols-3 gap-1.5">
            {own.map((picture) => (
              <CardPicture
                key={picture.id}
                picture={picture}
                onRole={(next) => role.mutate({ id: picture.id, next })}
                onRemove={() => detach.mutate(picture.id)}
              />
            ))}
          </div>
        )}
        {/* A strip under the gallery rather than a tile in it: a tile a third
            of the panel wide has no room for its words. */}
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void choose()}
          className="justify-start border border-dashed border-line-2 text-faint"
        >
          <ImagePlus aria-hidden />
          {t('canon.addPictureHint')}
        </Button>
      </section>

      {view.relations.length > 0 && (
        <section className="flex flex-col gap-1">
          <header className="flex items-center gap-2">
            <b className="text-sm font-semibold">{t('canon.around')}</b>
            <span className="text-2xs text-faint">{t('canon.aroundHint')}</span>
          </header>
          {view.relations.slice(0, 8).map((relation) => (
            <RowButton
              key={relation.link.id}
              onClick={() => onOpen(relation.other_id)}
              className="gap-2 px-1 py-0.5"
              start={<CardAvatar title={relation.other_title} portrait={null} size="sm" />}
              end={
                relation.label !== null ? (
                  <span className="truncate text-2xs">{relation.label}</span>
                ) : undefined
              }
            >
              {relation.other_title ?? t('canon.untitled')}
            </RowButton>
          ))}
        </section>
      )}

      {mine.length > 0 && <CanonProposals proposals={mine} />}

      <section className="flex flex-col gap-1.5">
        <b className="text-sm font-semibold">{t('canon.howRead')}</b>
        <p className="text-xs text-faint">{t('canon.howReadBody')}</p>
        <Button size="sm" variant="ghost" className="self-start" onClick={() => setSeeing(true)}>
          <Eye aria-hidden />
          {t('canon.seeAsTask')}
        </Button>
      </section>

      {seeing && (
        <SeenDialog
          open
          onOpenChange={setSeeing}
          cardId={card.id}
          lens={lens === 'all' ? 'work' : lens}
        />
      )}
    </Pane>
  )
}
