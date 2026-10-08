import { useMemo, type ReactNode } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { Copy, FolderOpen, FolderPlus, Play, RefreshCw } from 'lucide-react'
import { attachAsset } from '@/lib/api/assets'
import { createWorkFolder, openMedia } from '@/lib/api/folders'
import type { FolderFile, Work, WorkFolder } from '@/lib/api/types'
import { extensionOf, mediaKindOf } from '@/lib/media'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { MediaPreview } from '@/components/MediaPreview'
import { usePlayer } from '@/features/work/player'

/**
 * The work's folder on disk (v0.93, ADR 0057): what the person keeps for the
 * work outside kilna, found by the name the kind gives it under the media
 * folder rather than attached by hand.
 *
 * Looked at, never owned. Nothing here moves, renames or deletes a file: a
 * file is opened in the program the system opens it with, shown where it
 * lies, played in the card's player, or copied into the workspace - which is
 * the one way a file of the folder becomes the work's own.
 */
export function FolderSection({ work }: { work: Work }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const folder = useQuery(queries.folder(work.id))

  const make = useAppMutation({
    mutationFn: () => createWorkFolder(work.id),
    failure: 'folder.createFailed',
    refresh: [keys.folderFor(work.id)],
    onSuccess: () => say.ok(t('folder.created')),
  })

  const found = folder.data?.state === 'found'
  const path = folder.data?.path

  return (
    <section
      aria-label={t('folder.heading')}
      className="flex flex-col gap-2 border-t border-line pt-3"
    >
      <header className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
        <h3 className="caption">
          {t('folder.heading')}
          {found ? ` · ${String(folder.data?.files.length ?? 0)}` : ''}
        </h3>
        {path !== undefined && (
          <span className="min-w-0 truncate font-mono text-xs text-faint" title={path}>
            {path}
          </span>
        )}
        <span className="ml-auto flex items-center gap-1">
          {found && path !== undefined && (
            <Button
              size="xs"
              variant="ghost"
              onClick={() => {
                revealItemInDir(path).catch((cause: unknown) => say.failed(cause))
              }}
            >
              <FolderOpen aria-hidden />
              {t('folder.show')}
            </Button>
          )}
          <Button
            size="icon-xs"
            variant="icon"
            aria-label={t('folder.refresh')}
            title={t('folder.refresh')}
            disabled={folder.isFetching}
            onClick={() => void folder.refetch()}
          >
            <RefreshCw aria-hidden className={cn(folder.isFetching && 'animate-spin')} />
          </Button>
        </span>
      </header>

      {folder.isPending ? (
        <Skeleton className="h-26 w-full" />
      ) : folder.isError ? (
        <p className="text-sm text-bad">{t('folder.loadFailed')}</p>
      ) : (
        <FolderBody
          folder={folder.data}
          work={work}
          creating={make.isPending}
          onCreate={() => make.mutate()}
          onSettings={(section) => void navigate(`/settings/${section}`)}
        />
      )}
    </section>
  )
}

/** What stands under the heading: the files, or why there are none. */
function FolderBody({
  folder,
  work,
  creating,
  onCreate,
  onSettings,
}: {
  folder: WorkFolder
  work: Work
  creating: boolean
  onCreate: () => void
  onSettings: (section: string) => void
}) {
  const { t } = useTranslation()

  switch (folder.state) {
    case 'noRoot':
      return (
        <Said>
          {t('folder.noRoot')}{' '}
          <Button variant="link" onClick={() => onSettings('data')}>
            {t('folder.chooseRoot')}
          </Button>
        </Said>
      )
    case 'rootMissing':
      return <Said>{t('folder.rootMissing', { path: folder.root ?? '' })}</Said>
    case 'noTemplate':
      return (
        <Said>
          {t('folder.noTemplate')}{' '}
          <Button variant="link" onClick={() => onSettings('profile')}>
            {t('folder.nameIt')}
          </Button>
        </Said>
      )
    case 'unfilled':
      return (
        <Said>
          {t('folder.unfilled', {
            template: folder.template ?? '',
            placeholder: folder.unfilled ?? '',
          })}
        </Said>
      )
    case 'absent':
      return (
        <Said>
          {t('folder.absent')}{' '}
          <Button size="xs" variant="soft" disabled={creating} onClick={onCreate}>
            <FolderPlus aria-hidden />
            {t('folder.create')}
          </Button>
        </Said>
      )
    case 'found':
      return folder.files.length === 0 ? (
        <Said>{t('folder.empty')}</Said>
      ) : (
        <FolderFiles files={folder.files} truncated={folder.truncated} work={work} />
      )
  }
}

function Said({ children }: { children: ReactNode }) {
  return <p className="text-sm text-dim">{children}</p>
}

/** The files in groups by the folder they lie in, the work's own first. */
function FolderFiles({
  files,
  truncated,
  work,
}: {
  files: FolderFile[]
  truncated: boolean
  work: Work
}) {
  const { t } = useTranslation()
  const groups = useMemo(() => byFolder(files), [files])

  return (
    <div className="flex flex-col gap-3">
      {groups.map((group) => (
        <section key={group.folder} className="flex flex-col gap-2">
          <h4 className="caption font-mono normal-case">
            {group.folder === '' ? t('folder.top') : group.folder} · {group.files.length}
          </h4>
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-2.5">
            {group.files.map((file) => (
              <FolderTile key={file.path} file={file} work={work} />
            ))}
          </ul>
        </section>
      ))}
      {truncated && <Said>{t('folder.truncated', { count: files.length })}</Said>}
    </div>
  )
}

/** Files grouped by the folder they lie in, in the order they were sorted. */
export function byFolder(files: FolderFile[]): { folder: string; files: FolderFile[] }[] {
  const groups = new Map<string, FolderFile[]>()
  for (const file of files) {
    const cut = file.relative.lastIndexOf('/')
    const folder = cut === -1 ? '' : file.relative.slice(0, cut)
    const group = groups.get(folder)
    if (group) group.push(file)
    else groups.set(folder, [file])
  }
  // The work's own files first, then each folder by its name.
  return [...groups.entries()]
    .sort(([a], [b]) => (a === '' ? -1 : b === '' ? 1 : a.localeCompare(b)))
    .map(([folder, list]) => ({ folder, files: list }))
}

/**
 * One file of the folder: what it looks like, what it is called, and what can
 * be done with it - opened, played, shown where it lies, copied in.
 */
function FolderTile({ file, work }: { file: FolderFile; work: Work }) {
  const { t } = useTranslation()
  const player = usePlayer()
  const name = file.relative.split('/').at(-1) ?? file.relative
  const kind = mediaKindOf(file.path)

  const copyIn = useAppMutation({
    mutationFn: () => attachAsset(file.path, { work_id: work.id }),
    failure: 'files.attachFailed',
    refresh: [keys.assetsFor(work.id), keys.covers, keys.cardCounts(work.id)],
    onSuccess: () => say.ok(t('files.attached', { name })),
  })

  // A sound is played here, in the card; everything else opens where the
  // system opens it.
  const open = () => {
    if (kind === 'sound' && player !== null) {
      player.play(file.path)
      return
    }
    openMedia(file.path).catch((cause: unknown) => say.failed(cause))
  }

  return (
    <li className="group relative overflow-hidden rounded-md border border-line bg-raise">
      {/* eslint-disable-next-line dowel/no-raw-button -- a thumbnail: the picture is the control; no primitive draws a pressable picture (the Button's control height would flatten the well) */}
      <button
        type="button"
        className={cn(
          'flex h-26 w-full items-center justify-center bg-bg transition-colors hover:bg-soft',
          'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent',
        )}
        title={kind === 'sound' ? t('player.play', { name }) : t('folder.open', { name })}
        aria-label={kind === 'sound' ? t('player.play', { name }) : t('folder.open', { name })}
        onClick={open}
      >
        {kind === 'sound' ? (
          // A sound has nothing to look at: the way to hear it, and its kind.
          <span className="flex flex-col items-center gap-1.5 font-mono text-xs uppercase tracking-caption text-faint">
            <Play aria-hidden className="size-8 rounded-full bg-accent p-2 text-on-accent" />
            {extensionOf(file.path)}
          </span>
        ) : (
          <MediaPreview
            path={file.path}
            alt={name}
            className="max-h-full max-w-full object-contain"
          />
        )}
      </button>
      <p className="truncate px-2 py-1.5 text-xs" title={file.relative}>
        {name}
      </p>
      <span
        className={cn(
          'absolute top-1.5 right-1.5 flex gap-1',
          'opacity-0 group-hover:opacity-100 focus-within:opacity-100',
        )}
      >
        <Button
          variant="icon"
          size="icon-xs"
          title={t('folder.reveal')}
          aria-label={t('folder.reveal')}
          className="rounded-sm bg-raise/85 shadow-lift not-data-disabled:hover:bg-raise"
          onClick={() => {
            revealItemInDir(file.path).catch((cause: unknown) => say.failed(cause))
          }}
        >
          <FolderOpen aria-hidden />
        </Button>
        <Button
          variant="icon"
          size="icon-xs"
          title={t('folder.copyIn')}
          aria-label={t('folder.copyIn')}
          disabled={copyIn.isPending}
          className="rounded-sm bg-raise/85 shadow-lift not-data-disabled:hover:bg-raise"
          onClick={() => copyIn.mutate()}
        >
          <Copy aria-hidden />
        </Button>
      </span>
    </li>
  )
}
