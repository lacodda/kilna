import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { Image as ImageIcon, Paperclip, Star, Trash2 } from 'lucide-react'
import {
  attachAsset,
  detachAsset,
  listWorkAssets,
  type Asset,
  type Work,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { groupMaterials } from '@/lib/materials'
import { PICTURES } from '@/lib/media'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { MediaPreview } from '@/components/ui/MediaPreview'
import { CoverPrompt } from '@/components/card/CoverPrompt'
import { Skeleton } from '@/components/ui/Skeleton'

interface Props {
  work: Work
}

/** The kind an asset takes when it is the work's cover. */
const COVER = 'cover'

/** What the picker offers: the pictures a webview can show. */

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
  const client = useQueryClient()
  const [busy, setBusy] = useState(false)

  const files = useQuery({
    queryKey: keys.assetsFor(work.id),
    queryFn: () => listWorkAssets(work.id),
  })

  const refresh = () => {
    void client.invalidateQueries({ queryKey: keys.assetsFor(work.id) })
    void client.invalidateQueries({ queryKey: keys.covers })
    void client.invalidateQueries({ queryKey: keys.journal })
  }

  const attach = useMutation({
    mutationFn: ({ source, kind }: { source: string; kind?: string }) =>
      attachAsset(source, { work_id: work.id, kind }),
    onSuccess: (asset) => {
      refresh()
      say.ok(t('files.attached', { name: asset.original_name ?? '' }))
    },
    onError: (cause) => say.failedTo(t('files.attachFailed'), cause),
  })

  const detach = useMutation({
    mutationFn: (asset: Asset) => detachAsset(asset.id),
    onSuccess: () => {
      refresh()
      say.ok(t('files.detached'))
    },
    onError: (cause) => say.failedTo(t('files.detachFailed'), cause),
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
    <div
      className={cn(
        'flex flex-col gap-4 rounded-lg transition-colors',
        over && 'outline-2 outline-dashed outline-offset-4 outline-accent',
      )}
    >
      {/* Above the pictures, because writing the prompt and looking at what
          came back is one activity. Draws nothing for a kind whose covers are
          not written. */}
      <CoverPrompt work={work} />

      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" disabled={busy || attach.isPending} onClick={() => void pick(COVER)}>
          <ImageIcon aria-hidden className="size-3.5" />
          {cover === undefined ? t('files.setCover') : t('files.changeCover')}
        </Button>
        <Button
          variant="soft"
          size="sm"
          disabled={busy || attach.isPending}
          onClick={() => void pick()}
        >
          <Paperclip aria-hidden className="size-3.5" />
          {t('files.attach')}
        </Button>
        <span className="text-xs text-dim">
          {over ? t('files.dropHere') : t('files.copiedIn')}
        </span>
      </div>

      {files.isPending && <Skeleton className="h-32 w-full" />}
      {files.isError && (
        <p role="alert" className="text-sm text-bad">
          {t('toast.loadFailed')}
        </p>
      )}
      {files.data !== undefined && all.length === 0 && (
        <p className="text-sm text-dim">{t('files.empty')}</p>
      )}

      {/* Grouped by what a file is for, not one flat list. A board of fifty
          scenes with four candidates each puts two hundred pictures in here,
          and the cover used to be somewhere among them. */}
      {groupMaterials(all).map((group) => (
        <section key={group.kind} className="flex flex-col gap-2">
          <h3 className="text-2xs font-semibold uppercase tracking-caption text-faint">
            {t(`files.group.${group.kind}`)}
            <span className="ml-1.5 font-normal normal-case tracking-normal text-dim">
              {group.assets.length}
            </span>
          </h3>
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-3">
            {group.assets.map((asset) => (
              <FileCard
                key={asset.id}
                asset={asset}
                isCover={asset.id === cover?.id}
                onDetach={() => detach.mutate(asset)}
              />
            ))}
          </ul>
        </section>
      ))}
    </div>
  )
}

/**
 * One file: what it looks like, what it is called, and the way out.
 *
 * The picture is shown whole rather than cropped to fill — a cover may be
 * square, a reference may be a tall screenshot, and a gallery that crops
 * both to the same rectangle shows neither.
 */
function FileCard({
  asset,
  isCover,
  onDetach,
}: {
  asset: Asset
  isCover: boolean
  onDetach: () => void
}) {
  const { t } = useTranslation()

  return (
    <li className="flex flex-col gap-1.5">
      <div
        className={cn(
          'relative flex h-32 items-center justify-center overflow-hidden rounded-md border bg-soft/40',
          isCover ? 'border-accent' : 'border-line',
        )}
      >
        {/* A scene's clips are files of the work too, and drew as broken
            pictures here until each file was shown as what it is. */}
        <MediaPreview
          path={asset.path}
          alt={asset.original_name ?? asset.label ?? ''}
          className="max-h-full max-w-full object-contain"
        />
        {isCover && (
          <span className="absolute left-1.5 top-1.5 flex items-center gap-1 rounded-full bg-black/55 px-1.5 py-0.5 text-[10.5px] text-white/90">
            <Star aria-hidden className="size-3 fill-current" />
            {t('files.cover')}
          </span>
        )}
      </div>
      <div className="flex items-start gap-1">
        <span className="min-w-0 flex-1 truncate text-xs text-dim" title={asset.original_name ?? ''}>
          {asset.original_name ?? asset.label ?? asset.id}
        </span>
        <Button
          variant="danger"
          size="icon-sm"
          title={t('files.detach')}
          aria-label={t('files.detach')}
          onClick={onDetach}
        >
          <Trash2 aria-hidden className="size-3.5" />
        </Button>
      </div>
    </li>
  )
}
