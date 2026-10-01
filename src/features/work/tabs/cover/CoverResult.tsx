import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open, save } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { Download, ImagePlus, Star, Trash2 } from 'lucide-react'
import type { Asset, CoverView, Work } from '@/lib/api/types'
import {
  assetBytes,
  attachAsset,
  chooseCover,
  detachAsset,
  fileSrc,
  pasteAsset,
  savePicture,
} from '@/lib/api/assets'
import { composeWithMark } from '@/lib/compose'
import { landsOn, picturesAmong } from '@/lib/drop'
import { PICTURES } from '@/lib/media'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Panel, SectionLabel } from '@/components/ui/panel'

/** The kinds a picture of the Cover tab has: the final one, and the rest. */
const COVER = 'cover'
const CANDIDATE = 'candidate'

interface Props {
  work: Work
  view: CoverView
}

/**
 * What came back from the generator: the candidates, and the one chosen -
 * the release's preview, the picture the catalogue, the calendar and the
 * card's header show.
 *
 * Three ways in, as a scene's frames have: a file picked, a file dropped
 * from a folder onto the panel, and a picture pasted with Ctrl+V straight
 * from the generator's page. Each arrives as a candidate; one of them is
 * made the cover, and the cover it replaces is a candidate again.
 *
 * The cover is exported with the channel's mark laid over it in the corner
 * the constructor shows, when the mark goes on that way - the one picture
 * that leaves for the platform.
 */
export function CoverResult({ work, view }: Props) {
  const { t } = useTranslation()
  const region = useRef<HTMLDivElement>(null)
  const [over, setOver] = useState(false)
  const assets = useQuery(queries.assets(work.id))
  const REFRESHED = [keys.assetsFor(work.id), keys.covers, keys.cardCounts(work.id)] as const

  const pictures = (assets.data ?? []).filter(
    (asset) => asset.kind === COVER || asset.kind === CANDIDATE,
  )
  // The newest cover is the cover; an older one is a candidate in all but
  // its kind, from before a cover was chosen rather than attached.
  const final = [...pictures].reverse().find((asset) => asset.kind === COVER)
  const candidates = pictures.filter((asset) => asset !== final).reverse()

  const attach = useAppMutation({
    mutationFn: (source: string) => attachAsset(source, { work_id: work.id, kind: CANDIDATE }),
    refresh: REFRESHED,
    onSuccess: () => say.ok(t('cover.result.added')),
  })
  const paste = useAppMutation({
    mutationFn: ({ bytes, name }: { bytes: Uint8Array; name: string }) =>
      pasteAsset(bytes, name, { work_id: work.id, kind: CANDIDATE }),
    refresh: REFRESHED,
    onSuccess: () => say.ok(t('cover.result.pasted')),
  })
  const choose = useAppMutation({
    mutationFn: (id: string) => chooseCover(id),
    refresh: [...REFRESHED, keys.calendar, keys.catalogue, keys.works],
    onSuccess: () => say.ok(t('cover.result.chosen')),
  })
  const remove = useAppMutation({
    mutationFn: (id: string) => detachAsset(id),
    refresh: REFRESHED,
  })

  const pick = async () => {
    const chosen = await open({
      multiple: true,
      filters: [{ name: t('cover.result.pictures'), extensions: PICTURES }],
    })
    for (const source of Array.isArray(chosen) ? chosen : chosen === null ? [] : [chosen]) {
      attach.mutate(source)
    }
  }

  // A file dropped from a folder: the panel's, when the pointer is over it.
  useEffect(() => {
    let stop: (() => void) | undefined
    let alive = true
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload
        if (payload.type === 'leave') {
          setOver(false)
          return
        }
        const here = landsOn(
          region.current?.getBoundingClientRect(),
          payload.position,
          window.devicePixelRatio,
        )
        if (payload.type === 'over' || payload.type === 'enter') {
          setOver(here)
          return
        }
        setOver(false)
        if (payload.type === 'drop' && here) {
          const { pictures: dropped, others } = picturesAmong(payload.paths)
          for (const source of dropped) attach.mutate(source)
          if (others > 0) say.warn(t('cover.result.notPictures', { count: others }))
        }
      })
      .then((unlisten) => {
        if (alive) stop = unlisten
        else unlisten()
      })
    return () => {
      alive = false
      stop?.()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [work.id])

  // A picture pasted from the generator's page. The tab is open, so a
  // picture on the clipboard pasted anywhere in it is a candidate - unless
  // the focus is in a field, where a paste is text.
  useEffect(() => {
    const onPaste = async (event: ClipboardEvent) => {
      const target = event.target as HTMLElement | null
      if (target?.closest('input, textarea, [contenteditable="true"]')) return
      const file = event.clipboardData?.files?.[0]
      if (file === undefined || !file.type.startsWith('image/')) return
      event.preventDefault()
      try {
        const extension = file.type.split('/')[1] ?? 'png'
        const name = file.name || `pasted-${Date.now()}.${extension}`
        paste.mutate({ bytes: new Uint8Array(await file.arrayBuffer()), name })
      } catch (error) {
        say.failed(error)
      }
    }
    window.addEventListener('paste', onPaste)
    return () => window.removeEventListener('paste', onPaste)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [work.id])

  // The variant laid over at export, when the mark goes on that way.
  const laidOver =
    view.mark_box === null
      ? null
      : (view.marks.find((option) => option.id === work.cover.mark.variant) ?? null)

  return (
    <Panel role="region" aria-label={t('cover.result.title')} className="flex flex-col gap-2 p-3">
      <header className="flex flex-wrap items-center gap-2">
        <SectionLabel>{t('cover.result.title')}</SectionLabel>
        <span className="text-2xs text-faint">{t('cover.result.hint')}</span>
        <Button size="xs" className="ml-auto" onClick={pick} disabled={attach.isPending}>
          <ImagePlus />
          {t('cover.result.pick')}
        </Button>
      </header>
      <div
        ref={region}
        className={cn(
          'grid grid-cols-[repeat(auto-fill,minmax(7rem,1fr))] gap-2 rounded-md p-1',
          over && 'bg-accent-soft outline-2 outline-dashed outline-accent',
        )}
      >
        {final !== undefined && (
          <Picture
            asset={final}
            final
            onRemove={() => remove.mutate(final.id)}
            busy={remove.isPending || choose.isPending}
          />
        )}
        {candidates.map((asset) => (
          <Picture
            key={asset.id}
            asset={asset}
            final={false}
            onChoose={() => choose.mutate(asset.id)}
            onRemove={() => remove.mutate(asset.id)}
            busy={remove.isPending || choose.isPending}
          />
        ))}
        <p className="col-span-full rounded-md border border-dashed border-line-2 px-3 py-4 text-center text-xs text-faint">
          {t('cover.result.drop')}
        </p>
      </div>
      {final !== undefined && (
        <ExportButton work={work} view={view} final={final} mark={laidOver} />
      )}
    </Panel>
  )
}

function Picture({
  asset,
  final,
  busy,
  onChoose,
  onRemove,
}: {
  asset: Asset
  final: boolean
  busy: boolean
  onChoose?: () => void
  onRemove: () => void
}) {
  const { t } = useTranslation()
  return (
    <figure className="flex min-w-0 flex-col gap-1">
      <span
        className={cn(
          'relative block aspect-square overflow-hidden rounded-md bg-soft',
          final && 'ring-2 ring-accent',
        )}
      >
        <img
          src={fileSrc(asset.path)}
          alt={asset.original_name ?? ''}
          className="size-full object-cover"
          draggable={false}
        />
        {final && (
          <span className="absolute bottom-1 left-1 rounded-sm bg-accent px-1 text-2xs font-semibold text-on-accent">
            {t('cover.result.final')}
          </span>
        )}
      </span>
      <figcaption className="flex items-center gap-1">
        <span
          className="min-w-0 flex-1 truncate text-2xs text-faint"
          title={asset.original_name ?? ''}
        >
          {asset.original_name ?? asset.label ?? ''}
        </span>
        {onChoose !== undefined && (
          <Button
            variant="icon"
            size="icon-xs"
            disabled={busy}
            aria-label={t('cover.result.choose')}
            title={t('cover.result.choose')}
            onClick={onChoose}
          >
            <Star />
          </Button>
        )}
        <Button
          variant="icon"
          size="icon-xs"
          disabled={busy}
          aria-label={t('cover.result.remove')}
          title={t('cover.result.remove')}
          onClick={onRemove}
        >
          <Trash2 />
        </Button>
      </figcaption>
    </figure>
  )
}

/**
 * Save the cover for the platform: with the channel's mark laid over it when
 * the mark goes on that way, as it is otherwise.
 */
function ExportButton({
  work,
  view,
  final,
  mark,
}: {
  work: Work
  view: CoverView
  final: Asset
  mark: CoverView['marks'][number] | null
}) {
  const { t } = useTranslation()
  const [busy, setBusy] = useState(false)
  const file = mark?.file ?? null
  const box = view.mark_box

  const run = async () => {
    const stem = `${view.title || work.title} ${view.format.replace(':', 'x')}`.trim()
    const path = await save({
      defaultPath: `${stem}.png`,
      filters: [{ name: 'PNG', extensions: ['png'] }],
    })
    if (path === null) return
    setBusy(true)
    try {
      const picture = await assetBytes(final.id)
      let bytes: Uint8Array
      if (file !== null && box !== null) {
        const sign = await assetBytes(file.id)
        bytes = await composeWithMark(
          { bytes: picture, name: final.path },
          { bytes: sign, name: file.path },
          box,
        )
      } else {
        bytes = new Uint8Array(picture)
      }
      await savePicture(path, bytes)
      say.ok(t('cover.result.exported'))
    } catch (error) {
      say.failed(error)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Button size="sm" variant="primary" onClick={run} disabled={busy}>
        <Download />
        {file !== null && box !== null
          ? t('cover.result.exportWithMark')
          : t('cover.result.export')}
      </Button>
      {box !== null && file === null && (
        <span className="text-2xs text-warn">{t('cover.result.markHasNoFile')}</span>
      )}
    </div>
  )
}
