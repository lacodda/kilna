import { useMemo, type ReactNode } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { CircleCheck, Copy, FolderOpen, RefreshCw, Undo2 } from 'lucide-react'
import { attachAsset } from '@/lib/api/assets'
import { openMedia, openWorkFolder } from '@/lib/api/folders'
import type { FinishedFile, FolderFile, Work, WorkFolder } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { arrange, entryOfFolderFile, type FileEntry, type FilesLook } from '@/lib/fileList'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { InlineField, numberCodec } from '@/components/ui/inline-field'
import { Skeleton } from '@/components/ui/skeleton'
import { usePlayer } from '@/features/work/player'
import { FileEntries } from '@/features/work/tabs/files/FileEntries'

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
export function FolderSection({ work, look }: { work: Work; look: FilesLook }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const folder = useQuery(queries.folder(work.id))

  // Opening the folder is what makes it (FR-04): there is no folder until
  // one is needed, and no "make the folder" to press first. The listing is
  // read again, since a folder that was not there now is.
  const open = useAppMutation({
    mutationFn: () => openWorkFolder(work.id),
    failure: 'folder.openFolderFailed',
    refresh: [keys.folderFor(work.id)],
  })

  const found = folder.data?.state === 'found'
  const path = folder.data?.path
  // Somewhere to open: the folder is there, or would be.
  const openable = found || folder.data?.state === 'absent'

  return (
    <section
      aria-label={t('folder.heading')}
      className="flex flex-col gap-2 border-t border-line pt-3"
    >
      <header className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
        <h2 className="caption">
          {t('folder.heading')}
          {found ? ` · ${String(folder.data?.files.length ?? 0)}` : ''}
        </h2>
        {path !== undefined && (
          <span className="min-w-0 truncate font-mono text-xs text-faint" title={path}>
            {path}
          </span>
        )}
        {work.ordinal !== null && <Ordinal work={work} />}
        <span className="ml-auto flex items-center gap-1">
          {openable && (
            <Button
              size="xs"
              variant="ghost"
              disabled={open.isPending}
              onClick={() => open.mutate()}
            >
              <FolderOpen aria-hidden />
              {t('folder.openFolder')}
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

      {found && folder.data !== undefined && <FinishedLine work={work} file={folder.data.file} />}

      {folder.isPending ? (
        <Skeleton className="h-26 w-full" />
      ) : folder.isError ? (
        <p className="text-sm text-bad">{t('folder.loadFailed')}</p>
      ) : (
        <FolderBody
          folder={folder.data}
          work={work}
          look={look}
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
  look,
  onSettings,
}: {
  folder: WorkFolder
  work: Work
  look: FilesLook
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
      return <Said>{t('folder.absent')}</Said>
    case 'found':
      return folder.files.length === 0 ? (
        <Said>{t('folder.empty')}</Said>
      ) : (
        <FolderFiles
          files={folder.files}
          truncated={folder.truncated}
          work={work}
          finished={folder.file}
          look={look}
        />
      )
  }
}

function Said({ children }: { children: ReactNode }) {
  return <p className="text-sm text-dim">{children}</p>
}

/**
 * The work's number among the works of its kind made from the same origin -
 * the 2 of "Song (short 2)", which `{n}` reads in its folder (ADR 0070). Given
 * when the work was made; typed over when the folder on disk says otherwise.
 */
function Ordinal({ work }: { work: Work }) {
  const { t } = useTranslation()
  const renumber = useAppMutation({
    mutationFn: (ordinal: number) => updateWork(work.id, { ordinal }),
    refresh: refresh.finishedFile(work.id),
  })

  return (
    <span className="flex items-center gap-1 text-xs text-faint">
      {t('folder.number')}
      <InlineField
        className="w-12"
        label={t('folder.number')}
        labelHidden
        codec={numberCodec}
        value={work.ordinal}
        disabled={renumber.isPending}
        onCommit={(next) => {
          // A number is whole and above nought; anything else goes back to
          // the stored one, which is certainly true.
          if (next === null || !Number.isInteger(next) || next < 1 || next === work.ordinal) {
            return false
          }
          renumber.mutate(next)
        }}
      />
    </span>
  )
}

/**
 * Which file the work is finished as (ADR 0070): the newest the kind's rule
 * matches, or the one chosen by hand - and, for a choice, the way back to the
 * rule. What a cutter cuts a short from.
 */
function FinishedLine({ work, file }: { work: Work; file: FinishedFile }) {
  const { t } = useTranslation()
  const back = useAppMutation({
    mutationFn: () => updateWork(work.id, { file: null }),
    refresh: refresh.finishedFile(work.id),
  })
  const rule = file.rule ?? ''
  const said = finishedSaying(file, rule, t)
  if (said === null) return null

  return (
    <p
      className={cn(
        'flex flex-wrap items-center gap-x-2 text-sm',
        file.state === 'chosenMissing' ? 'text-warn' : 'text-dim',
      )}
    >
      <CircleCheck
        aria-hidden
        className={cn(
          'size-3.5 shrink-0',
          file.state === 'found' || file.state === 'chosen' ? 'text-good' : 'text-faint',
        )}
      />
      <span className="min-w-0">{said}</span>
      {(file.state === 'chosen' || file.state === 'chosenMissing') && (
        <Button size="xs" variant="ghost" disabled={back.isPending} onClick={() => back.mutate()}>
          <Undo2 aria-hidden />
          {t('folder.finished.backToRule')}
        </Button>
      )}
    </p>
  )
}

/** The line that says how the finished file was found, or why it was not. */
function finishedSaying(
  file: FinishedFile,
  rule: string,
  t: ReturnType<typeof useTranslation>['t'],
): string | null {
  switch (file.state) {
    case 'found':
      return t('folder.finished.found', { file: file.relative ?? '', rule })
    case 'chosen':
      return t('folder.finished.chosen', { file: file.relative ?? '' })
    case 'chosenMissing':
      return t('folder.finished.chosenMissing', { file: file.relative ?? '' })
    case 'notYet':
      return t('folder.finished.notYet', { rule })
    case 'noRule':
      return t('folder.finished.noRule')
    case 'noFolder':
      return null
  }
}

/**
 * The folder's files, drawn as the tab is arranged (FR-14): by what they are
 * or by the folder they lie in - the work's own first - in one of the four
 * views. Every folder below is read, however deep, and a file says which it
 * is in by the group it stands under or, gathered by type, by nothing: the
 * name is what is looked for there.
 */
function FolderFiles({
  files,
  truncated,
  work,
  finished,
  look,
}: {
  files: FolderFile[]
  truncated: boolean
  work: Work
  finished: FinishedFile
  look: FilesLook
}) {
  const { t } = useTranslation()
  const player = usePlayer()
  const entries = useMemo(
    () => files.map((file) => entryOfFolderFile(file, finished.path)),
    [files, finished.path],
  )
  const relativeOf = useMemo(
    () => new Map(files.map((file) => [file.path, file.relative])),
    [files],
  )
  const groups = arrange(entries, look)

  // A sound is played here, in the card; everything else opens where the
  // system opens it.
  const open = (entry: FileEntry) => {
    if (entry.category === 'audio' && player !== null) {
      player.play(entry.path)
      return
    }
    openMedia(entry.path).catch((cause: unknown) => say.failed(cause))
  }

  return (
    <div className="flex flex-col gap-3">
      {groups.length === 0 ? (
        <Said>{t('files.noneOfThese')}</Said>
      ) : (
        <FileEntries
          groups={groups}
          view={look.view}
          pathHeadings
          headingOf={(group) =>
            group.by === 'type'
              ? t(`files.category.${group.key}`)
              : group.key === ''
                ? t('folder.top')
                : group.key
          }
          onOpen={open}
          openLabel={(entry) =>
            entry.category === 'audio'
              ? t('player.play', { name: entry.name })
              : t('folder.open', { name: entry.name })
          }
          actions={(entry) => (
            <FolderActions
              entry={entry}
              relative={relativeOf.get(entry.path) ?? entry.name}
              work={work}
            />
          )}
        />
      )}
      {truncated && <Said>{t('folder.truncated', { count: files.length })}</Said>}
    </div>
  )
}

/**
 * What can be done with a file of the folder beyond opening it: shown where
 * it lies, made the work's finished file, copied into the workspace.
 */
function FolderActions({
  entry,
  relative,
  work,
}: {
  entry: FileEntry
  relative: string
  work: Work
}) {
  const { t } = useTranslation()
  // A clip or a sound can be what a work is finished as; a picture or a note
  // beside it cannot be cut or played as the work.
  const finishable = entry.category === 'video' || entry.category === 'audio'
  const finished = entry.roles.includes('finished')
  const onPicture = 'rounded-sm bg-raise/85 shadow-lift not-data-disabled:hover:bg-raise'

  const choose = useAppMutation({
    mutationFn: () => updateWork(work.id, { file: relative }),
    refresh: refresh.finishedFile(work.id),
    onSuccess: () => say.ok(t('folder.finished.madeIt', { name: entry.name })),
  })

  const copyIn = useAppMutation({
    mutationFn: () => attachAsset(entry.path, { work_id: work.id }),
    failure: 'files.attachFailed',
    refresh: [keys.assetsFor(work.id), keys.covers, keys.cardCounts(work.id)],
    onSuccess: () => say.ok(t('files.attached', { name: entry.name })),
  })

  return (
    <>
      <Button
        variant="icon"
        size="icon-xs"
        title={t('folder.reveal')}
        aria-label={t('folder.reveal')}
        className={onPicture}
        onClick={() => {
          revealItemInDir(entry.path).catch((cause: unknown) => say.failed(cause))
        }}
      >
        <FolderOpen aria-hidden />
      </Button>
      {finishable && !finished && (
        <Button
          variant="icon"
          size="icon-xs"
          title={t('folder.finished.makeIt')}
          aria-label={t('folder.finished.makeIt')}
          disabled={choose.isPending}
          className={onPicture}
          onClick={() => choose.mutate()}
        >
          <CircleCheck aria-hidden />
        </Button>
      )}
      <Button
        variant="icon"
        size="icon-xs"
        title={t('folder.copyIn')}
        aria-label={t('folder.copyIn')}
        disabled={copyIn.isPending}
        className={onPicture}
        onClick={() => copyIn.mutate()}
      >
        <Copy aria-hidden />
      </Button>
    </>
  )
}
