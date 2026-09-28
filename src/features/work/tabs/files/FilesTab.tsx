import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { Image as ImageIcon, Paperclip, X } from 'lucide-react'
import { attachAsset, detachAsset } from '@/lib/api/assets'
import type { Asset, Work } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { groupMaterials } from '@/lib/materials'
import { PICTURES } from '@/lib/media'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { ConfirmAction } from '@/components/ConfirmAction'
import { MediaPreview } from '@/components/MediaPreview'
import { EmptyState } from '@/components/ui/empty-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Frame, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { CoverPrompt } from '@/features/work/tabs/files/CoverPrompt'

interface Props {
  work: Work
}

/** The kind an asset takes when it is the work's cover. */
const COVER = 'cover'

/**
 * The files attached to a work: covers and references.
 *
 * A file is *copied* into the workspace rather than pointed at where it
 * lies, so a cover keeps working the day its source folder is tidied away,
 * and a backup carries the pictures with the database. What the person sees
 * is the name the file arrived under; what the workspace stores is a name of
 * its own, which nothing outside can collide with.
 */
export function FilesTab({ work }: Props) {
  const { t } = useTranslation()
  const [busy, setBusy] = useState(false)
  // The file whose removal is being asked about.
  const [leaving, setLeaving] = useState<Asset | null>(null)

  const files = useQuery(queries.assets(work.id))

  const attach = useAppMutation({
    mutationFn: ({ source, kind }: { source: string; kind?: string }) =>
      attachAsset(source, { work_id: work.id, kind }),
    failure: 'files.attachFailed',
    refresh: [keys.assetsFor(work.id), keys.covers],
    onSuccess: (asset) => say.ok(t('files.attached', { name: asset.original_name ?? '' })),
  })

  const detach = useAppMutation({
    mutationFn: (asset: Asset) => detachAsset(asset.id),
    failure: 'files.detachFailed',
    refresh: [keys.assetsFor(work.id), keys.covers],
    onSuccess: () => {
      setLeaving(null)
      say.ok(t('files.detached'))
    },
  })

  /** Ask for a file and attach it — the other way in is dropping one on the
      tab. */
  const pick = async (kind?: string) => {
    setBusy(true)
    try {
      const chosen = await open({
        multiple: false,
        filters: [{ name: t('files.pictures'), extensions: PICTURES }],
      })
      if (typeof chosen === 'string') attach.mutate({ source: chosen, kind })
    } catch (cause) {
      say.failedTo(t('files.attachFailed'), cause)
    } finally {
      setBusy(false)
    }
  }

  // Files dropped on the window. The event has no target — it belongs to the
  // whole window — so this listens only while the tab is open, and a drop
  // anywhere in it lands here. A person on another tab drops onto nothing,
  // which is better than a picture arriving on a work they were not looking
  // at.
  const [over, setOver] = useState(false)
  useEffect(() => {
    const subscription = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === 'over') {
        setOver(true)
        return
      }
      setOver(false)
      if (payload.type !== 'drop') return
      for (const source of payload.paths) attach.mutate({ source })
    })
    return () => {
      void subscription.then((unlisten) => {
        unlisten()
      })
    }
    // `attach` is stable enough for this: the mutation is recreated on every
    // render, and re-subscribing on each one would drop events mid-flight.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [work.id])

  const all = files.data ?? []
  // The newest cover is the cover (the backend answers the same way); the
  // ones it replaced stay attached rather than being deleted behind the
  // person's back.
  const cover = all.filter((asset) => asset.kind === COVER).at(-1)

  return (
    <Frame
      className={cn(
        'rounded-lg transition-colors',
        over && 'outline-2 outline-dashed outline-offset-4 outline-accent',
      )}
      head={
        // The prompt stands above the buttons, as the mockup has it: writing
        // it and looking at what came back are one activity, and the pictures
        // scroll under both. Draws nothing for a kind whose covers are not
        // written.
        <div className="flex w-full flex-col gap-2.5">
          <CoverPrompt work={work} />
          <div className="flex flex-wrap items-center gap-2">
            <Button size="sm" disabled={busy || attach.isPending} onClick={() => void pick(COVER)}>
              <ImageIcon aria-hidden />
              {cover === undefined ? t('files.setCover') : t('files.changeCover')}
            </Button>
            <Button size="sm" disabled={busy || attach.isPending} onClick={() => void pick()}>
              <Paperclip aria-hidden />
              {t('files.attach')}
            </Button>
            <span className="ml-auto text-xs text-faint">
              {over ? t('files.dropHere') : t('files.copiedIn')}
            </span>
          </div>
        </div>
      }
    >
      <Scroll label={t('card.tab.files')} contentClassName="flex flex-col gap-3">
        <Loaded
          query={files}
          skeleton={<Skeleton className="h-32 w-full" />}
          isEmpty={(data) => data.length === 0}
          // Plain: the way out - attaching one - is right above it.
          emptyState={<EmptyState plain title={t('files.empty')} />}
          plain
        >
          {(data) => (
            <div className="flex flex-col gap-3">
              {/* Grouped by what a file is for, not one flat list. A board of
                  fifty scenes with four candidates each puts two hundred
                  pictures in here, and the cover used to be somewhere among
                  them. */}
              {groupMaterials(data).map((group) => (
                <section key={group.kind} className="flex flex-col gap-2">
                  <h3 className="caption">
                    {t(`files.group.${group.kind}`)} · {group.assets.length}
                  </h3>
                  <ul className="grid grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-2.5">
                    {group.assets.map((asset) => (
                      <FileTile
                        key={asset.id}
                        asset={asset}
                        isCover={asset.id === cover?.id}
                        onDetach={() => setLeaving(asset)}
                      />
                    ))}
                  </ul>
                </section>
              ))}
            </div>
          )}
        </Loaded>
      </Scroll>

      {/* Asked, because nothing brings it back: the file leaves the workspace
          with its row, outside the trash and outside the undo. The one
          question the app puts to a removal (see ConfirmAction). */}
      <ConfirmAction
        open={leaving !== null}
        onOpenChange={(asking) => {
          if (!asking) setLeaving(null)
        }}
        title={t('files.detachTitle', { name: nameOf(leaving) })}
        description={t('files.detachBody')}
        actionLabel={t('files.detachConfirm')}
        pending={detach.isPending}
        onConfirm={() => {
          if (leaving !== null) detach.mutate(leaving)
        }}
      />
    </Frame>
  )
}

/** What a file is called on screen: the name it arrived under. */
function nameOf(asset: Asset | null): string {
  return asset === null ? '' : (asset.original_name ?? asset.label ?? asset.id)
}

/**
 * One file: what it looks like, what it is called, and the way out.
 *
 * The mockup's tile: a picture well of a fixed height with the name under it,
 * the cover marked by a tag in its corner, and the way out in the other
 * corner - shown when the pointer is over the tile, or when the keyboard
 * reaches it, rather than a red cross standing on every picture of a board.
 *
 * The picture is shown whole rather than cropped to fill — a cover may be
 * square, a reference may be a tall screenshot, and a gallery that crops
 * both to the same rectangle shows neither.
 */
function FileTile({
  asset,
  isCover,
  onDetach,
}: {
  asset: Asset
  isCover: boolean
  onDetach: () => void
}) {
  const { t } = useTranslation()
  const name = nameOf(asset)

  return (
    <li className="group relative overflow-hidden rounded-md border border-line bg-raise">
      <div className="flex h-26 items-center justify-center bg-bg">
        {/* A scene's clips are files of the work too, and drew as broken
            pictures here until each file was shown as what it is. */}
        <MediaPreview
          path={asset.path}
          alt={asset.original_name ?? asset.label ?? ''}
          className="max-h-full max-w-full object-contain"
        />
      </div>
      <p className="truncate px-2 py-1.5 text-xs" title={name}>
        {name}
      </p>
      {isCover && (
        <span className="absolute top-1.5 left-1.5 rounded-xs bg-accent px-1.5 text-2xs font-bold tracking-wide text-on-accent">
          {t('files.cover')}
        </span>
      )}
      <Button
        variant="icon"
        size="icon-xs"
        title={t('files.detach')}
        aria-label={t('files.detach')}
        onClick={onDetach}
        className={cn(
          // On the picture, so it carries a ground of its own: the theme's
          // raised surface, which reads over a dark still and a light one.
          'absolute top-1.5 right-1.5 rounded-sm bg-raise/85 shadow-lift',
          'not-data-disabled:hover:bg-raise',
          'opacity-0 group-hover:opacity-100 focus-visible:opacity-100',
        )}
      >
        <X aria-hidden />
      </Button>
    </li>
  )
}
