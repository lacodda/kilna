import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { Image as ImageIcon, Paperclip, Upload, X } from 'lucide-react'
import { attachAsset, chooseCover, detachAsset } from '@/lib/api/assets'
import type { Asset, Work } from '@/lib/api/types'
import {
  arrange,
  ATTACHED_PLACES,
  CATEGORIES,
  categoryOf,
  entryOfAsset,
  loadLook,
  opensInSystem,
  saveLook,
  sightOf,
  type FileCategory,
  type FileEntry,
  type FilesLook,
} from '@/lib/fileList'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { PICTURES } from '@/lib/media'
import { cn } from '@/lib/utils'
import { hasDoors, useProfile, vocabularyOf } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { ConfirmAction } from '@/components/ConfirmAction'
import { Skeleton } from '@/components/ui/skeleton'
import { Frame } from '@/components/ui/frame'
import { Scroll } from '@/components/Scroll'
import { Loaded } from '@/components/Loaded'
import { usePlayer } from '@/features/work/player'
import type { RowAction } from '@/components/RowMenu'
import { FileEntries } from '@/features/work/tabs/files/FileEntries'
import { FileViewer } from '@/features/work/tabs/files/FileViewer'
import { FilesToolbar } from '@/features/work/tabs/files/FilesToolbar'
import { FolderSection } from '@/features/work/tabs/files/FolderSection'
import { useFileGestures } from '@/features/work/tabs/files/useFileGestures'

interface Props {
  work: Work
}

/** The kind an asset takes when it is the work's cover. */
const COVER = 'cover'

/**
 * The files of a work: the ones attached to it, and the ones in its folder on
 * disk.
 *
 * An attached file is *copied* into the workspace rather than pointed at
 * where it lies, so a cover keeps working the day its source folder is tidied
 * away, and a backup carries the pictures with the database. What the person
 * sees is the name the file arrived under; what the workspace stores is a
 * name of its own, which nothing outside can collide with.
 *
 * The folder on disk is the other arrangement (v0.93, ADR 0057): found by the
 * name the kind gives it, looked at and never owned - see `FolderSection`.
 */
export function FilesTab({ work }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  // A cover is what a work going out wears; a song, which goes out only as
  // what is made from it, has its folder here and no cover to set.
  const covered = hasDoors(config, work.kind) || vocabularyOf(config, work.kind).cover
  const [busy, setBusy] = useState(false)
  // The file whose removal is being asked about.
  const [leaving, setLeaving] = useState<Asset | null>(null)
  // How the tab is arranged: the person's, kept on this machine, and one
  // for the attached files and the folder on disk alike.
  const [look, setLook] = useState<FilesLook>(() => loadLook())
  const arrangeAs = (next: FilesLook) => {
    setLook(next)
    saveLook(next)
  }
  const player = usePlayer()
  const gestures = useFileGestures()
  // The attached file open in the viewer, by its key.
  const [viewing, setViewing] = useState<string | null>(null)

  const files = useQuery(queries.assets(work.id))
  // Read here as well as in the section that draws it, from one cache: the
  // bar counts every file of the work, wherever it lies.
  const folder = useQuery(queries.folder(work.id))

  // Any file, and as many as were picked or dropped (FR-14): a text, a
  // subtitle track, an archive is a file of the work as much as a picture
  // is. One toast for the lot - forty files dropped are not forty toasts.
  const attach = useAppMutation({
    mutationFn: async ({ sources, kind }: { sources: string[]; kind?: string }) => {
      const attached: Asset[] = []
      for (const source of sources) {
        attached.push(await attachAsset(source, { work_id: work.id, kind }))
      }
      return attached
    },
    failure: 'files.attachFailed',
    refresh: [keys.assetsFor(work.id), keys.covers, keys.cardCounts(work.id)],
    onSuccess: (attached) => {
      const [only] = attached
      if (only === undefined) return
      say.ok(
        attached.length === 1
          ? t('files.attached', { name: only.original_name ?? '' })
          : t('files.attachedMany', { count: attached.length }),
      )
    },
  })

  const detach = useAppMutation({
    mutationFn: (asset: Asset) => detachAsset(asset.id),
    failure: 'files.detachFailed',
    refresh: [keys.assetsFor(work.id), keys.covers, keys.cardCounts(work.id)],
    onSuccess: () => {
      setLeaving(null)
      say.ok(t('files.detached'))
    },
  })

  // An attached picture made the cover: the one the work wears from now on.
  const wear = useAppMutation({
    mutationFn: (assetId: string) => chooseCover(assetId),
    failure: 'files.attachFailed',
    refresh: [keys.assetsFor(work.id), keys.covers],
    onSuccess: () => say.ok(t('files.menu.coverSet')),
  })

  /** Ask for files and attach them - the other way in is dropping them on
      the tab. A cover is one picture; anything else is any files at all. */
  const pick = async (kind?: string) => {
    setBusy(true)
    try {
      const chosen =
        kind === COVER
          ? await open({
              multiple: false,
              filters: [{ name: t('files.pictures'), extensions: PICTURES }],
            })
          : await open({ multiple: true })
      const sources = chosen === null ? [] : Array.isArray(chosen) ? chosen : [chosen]
      if (sources.length > 0) attach.mutate({ sources, kind })
    } catch (cause) {
      say.failedTo(t('files.attachFailed'), cause)
    } finally {
      setBusy(false)
    }
  }

  // Files dropped on the window. The event has no target - it belongs to the
  // whole window - so this listens only while the tab is open, and a drop
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
      if (payload.type !== 'drop' || payload.paths.length === 0) return
      attach.mutate({ sources: payload.paths })
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

  const all = useMemo(() => files.data ?? [], [files.data])
  // The newest cover is the cover (the backend answers the same way); the
  // ones it replaced stay attached rather than being deleted behind the
  // person's back.
  const cover = all.filter((asset) => asset.kind === COVER).at(-1)
  const entries = useMemo(
    () => all.map((asset) => entryOfAsset(asset, cover?.id)),
    [all, cover?.id],
  )
  const groups = arrange(entries, look, ATTACHED_PLACES)

  const counts = useMemo(() => {
    const counted = Object.fromEntries(CATEGORIES.map((one) => [one, 0])) as Record<
      FileCategory,
      number
    >
    for (const entry of entries) counted[entry.category] += 1
    for (const file of folder.data?.files ?? []) counted[categoryOf(file.path)] += 1
    return counted
  }, [entries, folder.data?.files])

  // A sound is played here, in the card's player; everything else is looked
  // at here too, in the viewer (FR-14) - the program the system opens it
  // with is one press further, for working on it.
  const openEntry = (entry: FileEntry) => {
    if (sightOf(entry) === 'sound' && player !== null) {
      player.play(entry.path)
      return
    }
    setViewing(entry.key)
  }
  const openLabel = (entry: FileEntry) =>
    sightOf(entry) === 'sound'
      ? t('player.play', { name: entry.name })
      : t('files.menu.viewNamed', { name: entry.name })

  // Everything an attached file can have done to it, by name: what a right
  // click on it offers.
  const menuOf = (entry: FileEntry): RowAction[] => [
    {
      key: 'view',
      label: sightOf(entry) === 'sound' ? t('files.menu.play') : t('files.menu.view'),
      onSelect: () => openEntry(entry),
    },
    ...(opensInSystem(entry.name)
      ? [
          {
            key: 'outside',
            label: t('viewer.openOutside'),
            onSelect: () => gestures.openOutside(entry),
          },
        ]
      : []),
    { key: 'reveal', label: t('folder.reveal'), onSelect: () => gestures.reveal(entry) },
    { key: 'save', label: t('files.menu.saveCopy'), onSelect: () => gestures.saveCopy(entry) },
    { key: 'path', label: t('files.menu.copyPath'), onSelect: () => gestures.copyPath(entry) },
    ...(covered && entry.category === 'image' && !entry.roles.includes('cover')
      ? [
          {
            key: 'cover',
            label: t('files.menu.setCover'),
            onSelect: () => wear.mutate(entry.key),
          },
        ]
      : []),
    {
      key: 'remove',
      label: t('files.detach'),
      danger: true,
      onSelect: () => setLeaving(all.find((asset) => asset.id === entry.key) ?? null),
    },
  ]

  return (
    <Frame
      className={cn(
        'rounded-lg transition-colors',
        over && 'outline-2 outline-dashed outline-offset-4 outline-accent',
      )}
      head={
        // The cover's prompt has a tab of its own since v0.86 (`tabs/cover`);
        // what stands here is what adds a file, and how the files are
        // arranged.
        <div className="flex w-full flex-col gap-2">
          <div className="flex w-full flex-wrap items-center gap-2">
            {covered && (
              <Button
                size="sm"
                disabled={busy || attach.isPending}
                onClick={() => void pick(COVER)}
              >
                <ImageIcon aria-hidden />
                {cover === undefined ? t('files.setCover') : t('files.changeCover')}
              </Button>
            )}
            <Button size="sm" disabled={busy || attach.isPending} onClick={() => void pick()}>
              <Paperclip aria-hidden />
              {t('files.attach')}
            </Button>
            <span className="ml-auto text-xs text-faint">
              {over ? t('files.dropHere') : t('files.copiedIn')}
            </span>
          </div>
          <FilesToolbar look={look} onChange={arrangeAs} counts={counts} />
        </div>
      }
    >
      <Scroll label={t('card.tab.files')} contentClassName="flex flex-col gap-3">
        <Loaded
          query={files}
          skeleton={<Skeleton className="h-32 w-full" />}
          isEmpty={(data) => data.length === 0}
          // Nothing attached: the place to drop the first files, said as
          // one - until v0.121 an empty tab only said it was empty, and
          // that files could be dropped on it was something to find out.
          emptyState={
            <DropZone
              over={over}
              hint={covered ? t('files.empty') : t('files.emptyNoCover')}
              disabled={busy || attach.isPending}
              onPick={() => void pick()}
            />
          }
          plain
        >
          {() =>
            groups.length === 0 ? (
              <p className="text-sm text-dim">{t('files.noneOfThese')}</p>
            ) : (
              <FileEntries
                groups={groups}
                view={look.view}
                headingOf={(group) =>
                  group.by === 'type'
                    ? t(`files.category.${group.key}`)
                    : t(`files.group.${group.key}`)
                }
                onOpen={openEntry}
                openLabel={openLabel}
                menu={menuOf}
                playing={player?.selected}
                actions={(entry) => (
                  <Button
                    variant="icon"
                    size="icon-xs"
                    title={t('files.detach')}
                    aria-label={t('files.detach')}
                    // On the picture, so it carries a ground of its own: the
                    // theme's raised surface, which reads over a dark still
                    // and a light one.
                    className="rounded-sm bg-raise/85 shadow-lift not-data-disabled:hover:bg-raise"
                    onClick={() => setLeaving(all.find((asset) => asset.id === entry.key) ?? null)}
                  >
                    <X aria-hidden />
                  </Button>
                )}
              />
            )
          }
        </Loaded>
        <FolderSection work={work} look={look} />
      </Scroll>

      <FileViewer
        entries={groups.flatMap((group) => group.entries)}
        current={viewing}
        onChange={setViewing}
        onOpenOutside={gestures.openOutside}
        onReveal={gestures.reveal}
      />

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
 * Where the first files are dropped (FR-14): a place drawn as one, with the
 * other way in beside it. It takes any files, several at once; while files
 * are held over the window it says so.
 */
function DropZone({
  over,
  hint,
  disabled,
  onPick,
}: {
  over: boolean
  hint: string
  disabled: boolean
  onPick: () => void
}) {
  const { t } = useTranslation()
  return (
    <div
      className={cn(
        'flex flex-col items-center gap-2 rounded-lg border border-dashed px-4 py-8 text-center transition-colors',
        over ? 'border-accent bg-soft' : 'border-line-2',
      )}
    >
      <Upload aria-hidden className="size-6 text-faint" />
      <p className="text-sm font-semibold">{over ? t('files.dropHere') : t('files.dropZone')}</p>
      <p className="max-w-prose text-xs text-dim">{hint}</p>
      <Button size="sm" disabled={disabled} onClick={onPick}>
        <Paperclip aria-hidden />
        {t('files.attach')}
      </Button>
    </div>
  )
}
