import { useMemo, useState, type ReactNode } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { CircleCheck, Copy, FolderOpen, RefreshCw, Undo2 } from 'lucide-react'
import { attachAsset } from '@/lib/api/assets'
import { openWorkFolder, renameFolderFile, trashFolderFile } from '@/lib/api/folders'
import type { FinishedFile, FolderFile, Work, WorkFolder } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import {
  arrange,
  entryOfFolderFile,
  opensInSystem,
  sightOf,
  type FileEntry,
  type FilesLook,
} from '@/lib/fileList'
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
import { hasDoors, useProfile, vocabularyOf } from '@/lib/useProfile'
import { PromptDialog } from '@/components/AppDialog'
import { ConfirmAction } from '@/components/ConfirmAction'
import type { RowAction } from '@/components/RowMenu'
import { FileEntries } from '@/features/work/tabs/files/FileEntries'
import { FileViewer } from '@/features/work/tabs/files/FileViewer'
import { useFileGestures } from '@/features/work/tabs/files/useFileGestures'

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
  const { config } = useProfile()
  const player = usePlayer()
  const gestures = useFileGestures()
  const covered = hasDoors(config, work.kind) || vocabularyOf(config, work.kind).cover
  // The file open in the viewer, the one being renamed, the one on its way
  // to the recycle bin.
  const [viewing, setViewing] = useState<string | null>(null)
  const [renaming, setRenaming] = useState<FileEntry | null>(null)
  const [binning, setBinning] = useState<FileEntry | null>(null)

  const entries = useMemo(
    () => files.map((file) => entryOfFolderFile(file, finished.path)),
    [files, finished.path],
  )
  const relativeOf = useMemo(
    () => new Map(files.map((file) => [file.path, file.relative])),
    [files],
  )
  const relative = (entry: FileEntry) => relativeOf.get(entry.path) ?? entry.name
  const groups = arrange(entries, look)

  const rename = useAppMutation({
    mutationFn: ({ entry, name }: { entry: FileEntry; name: string }) =>
      renameFolderFile(work.id, relative(entry), name),
    failure: 'files.menu.renameFailed',
    // The finished file follows its name, so the work is read again too.
    refresh: refresh.finishedFile(work.id),
    onSuccess: () => say.ok(t('files.menu.renamed')),
  })
  const bin = useAppMutation({
    mutationFn: (entry: FileEntry) => trashFolderFile(work.id, relative(entry)),
    failure: 'files.menu.toBinFailed',
    refresh: refresh.finishedFile(work.id),
    onSuccess: () => {
      setBinning(null)
      say.ok(t('files.menu.binned'))
    },
  })
  const choose = useAppMutation({
    mutationFn: (entry: FileEntry) => updateWork(work.id, { file: relative(entry) }),
    refresh: refresh.finishedFile(work.id),
    onSuccess: (_, entry) => say.ok(t('folder.finished.madeIt', { name: entry.name })),
  })
  const copyIn = useAppMutation({
    mutationFn: ({ entry, kind }: { entry: FileEntry; kind?: string }) =>
      attachAsset(entry.path, { work_id: work.id, kind }),
    failure: 'files.attachFailed',
    refresh: [keys.assetsFor(work.id), keys.covers, keys.cardCounts(work.id)],
    onSuccess: (_, { entry, kind }) =>
      say.ok(
        kind === 'cover' ? t('files.menu.coverSet') : t('files.attached', { name: entry.name }),
      ),
  })

  // A sound is played here, in the card's player; everything else is looked
  // at in the viewer.
  const open = (entry: FileEntry) => {
    if (sightOf(entry) === 'sound' && player !== null) {
      player.play(entry.path)
      return
    }
    setViewing(entry.key)
  }

  // Everything a file of the folder can have done to it, by name. Renaming
  // and the recycle bin are the two things kilna does to a file it otherwise
  // only looks at, and each is asked for here and nowhere else.
  const menuOf = (entry: FileEntry): RowAction[] => {
    const finishable = entry.category === 'video' || entry.category === 'audio'
    const optional = (wanted: boolean, action: RowAction) => (wanted ? [action] : [])
    return [
      {
        key: 'view',
        label: sightOf(entry) === 'sound' ? t('files.menu.play') : t('files.menu.view'),
        onSelect: () => open(entry),
      },
      ...optional(opensInSystem(entry.name), {
        key: 'outside',
        label: t('viewer.openOutside'),
        onSelect: () => gestures.openOutside(entry),
      }),
      { key: 'reveal', label: t('folder.reveal'), onSelect: () => gestures.reveal(entry) },
      { key: 'rename', label: t('files.menu.rename'), onSelect: () => setRenaming(entry) },
      { key: 'save', label: t('files.menu.saveCopy'), onSelect: () => gestures.saveCopy(entry) },
      { key: 'path', label: t('files.menu.copyPath'), onSelect: () => gestures.copyPath(entry) },
      ...optional(finishable && !entry.roles.includes('finished'), {
        key: 'finished',
        label: t('folder.finished.makeIt'),
        onSelect: () => choose.mutate(entry),
      }),
      ...optional(covered && entry.category === 'image', {
        key: 'cover',
        label: t('files.menu.setCover'),
        onSelect: () => copyIn.mutate({ entry, kind: 'cover' }),
      }),
      { key: 'copyIn', label: t('folder.copyIn'), onSelect: () => copyIn.mutate({ entry }) },
      {
        key: 'bin',
        label: t('files.menu.toBin'),
        danger: true,
        onSelect: () => setBinning(entry),
      },
    ]
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
            sightOf(entry) === 'sound'
              ? t('player.play', { name: entry.name })
              : t('files.menu.viewNamed', { name: entry.name })
          }
          menu={menuOf}
          playing={player?.selected}
          actions={(entry) => (
            <FolderActions
              entry={entry}
              onReveal={() => gestures.reveal(entry)}
              onFinished={() => choose.mutate(entry)}
              onCopyIn={() => copyIn.mutate({ entry })}
              busy={choose.isPending || copyIn.isPending}
            />
          )}
        />
      )}
      {truncated && <Said>{t('folder.truncated', { count: files.length })}</Said>}

      <FileViewer
        entries={groups.flatMap((group) => group.entries)}
        current={viewing}
        onChange={setViewing}
        onOpenOutside={gestures.openOutside}
        onReveal={gestures.reveal}
      />

      {/* A name, not a place: the file stays in the folder it lies in, and
          a name that is taken is refused rather than written over. */}
      <PromptDialog
        open={renaming !== null}
        onOpenChange={(asking) => {
          if (!asking) setRenaming(null)
        }}
        title={t('files.menu.renameTitle', { name: renaming?.name ?? '' })}
        label={t('files.menu.renameLabel')}
        initialValue={renaming?.name ?? ''}
        confirmLabel={t('files.menu.renameConfirm')}
        onSubmit={(name) => {
          if (renaming !== null && name !== renaming.name) rename.mutate({ entry: renaming, name })
        }}
      />

      {/* Asked, because it leaves the folder: to the system's recycle bin,
          where it can be brought back from - kilna's own undo does not
          reach a file on disk. */}
      <ConfirmAction
        open={binning !== null}
        onOpenChange={(asking) => {
          if (!asking) setBinning(null)
        }}
        title={t('files.menu.toBinTitle', { name: binning?.name ?? '' })}
        description={t('files.menu.toBinBody')}
        actionLabel={t('files.menu.toBinConfirm')}
        pending={bin.isPending}
        onConfirm={() => {
          if (binning !== null) bin.mutate(binning)
        }}
      />
    </div>
  )
}

/**
 * The three things done most with a file of the folder, a press away on the
 * file itself: shown where it lies, made the work's finished file, copied
 * into the workspace. The rest are in its menu.
 */
function FolderActions({
  entry,
  onReveal,
  onFinished,
  onCopyIn,
  busy,
}: {
  entry: FileEntry
  onReveal: () => void
  onFinished: () => void
  onCopyIn: () => void
  busy: boolean
}) {
  const { t } = useTranslation()
  // A clip or a sound can be what a work is finished as; a picture or a note
  // beside it cannot be cut or played as the work.
  const finishable = entry.category === 'video' || entry.category === 'audio'
  const finished = entry.roles.includes('finished')
  const onPicture = 'rounded-sm bg-raise/85 shadow-lift not-data-disabled:hover:bg-raise'

  return (
    <>
      <Button
        variant="icon"
        size="icon-xs"
        title={t('folder.reveal')}
        aria-label={t('folder.reveal')}
        className={onPicture}
        onClick={onReveal}
      >
        <FolderOpen aria-hidden />
      </Button>
      {finishable && !finished && (
        <Button
          variant="icon"
          size="icon-xs"
          title={t('folder.finished.makeIt')}
          aria-label={t('folder.finished.makeIt')}
          disabled={busy}
          className={onPicture}
          onClick={onFinished}
        >
          <CircleCheck aria-hidden />
        </Button>
      )}
      <Button
        variant="icon"
        size="icon-xs"
        title={t('folder.copyIn')}
        aria-label={t('folder.copyIn')}
        disabled={busy}
        className={onPicture}
        onClick={onCopyIn}
      >
        <Copy aria-hidden />
      </Button>
    </>
  )
}
