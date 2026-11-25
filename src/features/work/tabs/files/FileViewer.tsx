import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ChevronLeft, ChevronRight, ExternalLink, FolderOpen, Minus, Plus, X } from 'lucide-react'
import { fileSrc } from '@/lib/api/assets'
import { readTextFile } from '@/lib/api/folders'
import { neighbour, opensInSystem, sightOf, type FileEntry } from '@/lib/fileList'
import { formatBytes, formatMoment } from '@/lib/format'
import { humanError } from '@/lib/errors'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { CopyButton } from '@/components/ui/copy-button'
import {
  Dialog,
  DialogActions,
  DialogBody,
  DialogClose,
  DialogHeader,
  DialogPopup,
  DialogTitle,
} from '@/components/ui/dialog'
import { Skeleton } from '@/components/ui/skeleton'
import { Select } from '@/components/AppSelect'

interface Props {
  /** The files being looked through, in the order the tab shows them. */
  entries: readonly FileEntry[]
  /** The one that is open, by its key; null when the viewer is shut. */
  current: string | null
  onChange: (key: string | null) => void
  /** Open it in the program the system opens it with. */
  onOpenOutside: (entry: FileEntry) => void
  /** Show it where it lies, in the file manager. */
  onReveal: (entry: FileEntry) => void
}

/** The steps a picture is enlarged by. */
const ZOOMS = [0.25, 0.5, 0.75, 1, 1.5, 2, 3, 4] as const
/** How fast a clip may play. */
const SPEEDS = ['0.5', '0.75', '1', '1.25', '1.5', '2'] as const

/**
 * A file, looked at inside kilna (FR-14): a picture at its size and larger,
 * a clip with its sound, a text to read and copy, a document - and, for what
 * the window cannot draw, what is known of it and the way out to the program
 * that can.
 *
 * Until v0.122 a click on a file opened it in another program, which is the
 * right place to work on a file and the wrong one to look at forty takes in
 * a row: the arrows page through the files as the tab shows them, without
 * the window being left. A sound is not here at all - it plays in the card's
 * player, where it keeps playing while the tab is looked through.
 */
export function FileViewer({ entries, current, onChange, onOpenOutside, onReveal }: Props) {
  const { t } = useTranslation()
  const entry = entries.find((one) => one.key === current)
  const before = entry === undefined ? undefined : neighbour(entries, entry.key, -1)
  const after = entry === undefined ? undefined : neighbour(entries, entry.key, 1)

  return (
    <Dialog
      open={entry !== undefined}
      onOpenChange={(open) => {
        if (!open) onChange(null)
      }}
    >
      <DialogPopup
        size="full"
        // The arrows page through the files; inside a control that uses them
        // - the speed of a clip, a text being selected - they are its own.
        onKeyDown={(event) => {
          const inControl = (event.target as HTMLElement).closest('select, input, video, pre')
          if (inControl !== null) return
          if (event.key === 'ArrowLeft' && before !== undefined) onChange(before.key)
          if (event.key === 'ArrowRight' && after !== undefined) onChange(after.key)
        }}
      >
        {entry !== undefined && (
          <>
            <DialogHeader
              action={
                <DialogClose
                  render={<Button variant="icon" size="icon-sm" aria-label={t('dialog.close')} />}
                >
                  <X aria-hidden />
                </DialogClose>
              }
            >
              <DialogTitle className="truncate">{entry.name}</DialogTitle>
            </DialogHeader>
            <DialogBody className="flex min-h-0 flex-col gap-2 py-1.5">
              {/* Keyed by the file, so a picture opens at its own size and a
                  clip at its own beginning rather than at the last one's. */}
              <Sight key={entry.key} entry={entry} />
            </DialogBody>
            <DialogActions
              start={
                <span className="flex items-center gap-1">
                  <Button
                    variant="icon"
                    size="icon-sm"
                    aria-label={t('viewer.previous')}
                    title={t('viewer.previous')}
                    disabled={before === undefined}
                    onClick={() => before !== undefined && onChange(before.key)}
                  >
                    <ChevronLeft aria-hidden />
                  </Button>
                  <Button
                    variant="icon"
                    size="icon-sm"
                    aria-label={t('viewer.next')}
                    title={t('viewer.next')}
                    disabled={after === undefined}
                    onClick={() => after !== undefined && onChange(after.key)}
                  >
                    <ChevronRight aria-hidden />
                  </Button>
                </span>
              }
            >
              <Button onClick={() => onReveal(entry)}>
                <FolderOpen aria-hidden />
                {t('folder.reveal')}
              </Button>
              {opensInSystem(entry.name) && (
                <Button onClick={() => onOpenOutside(entry)}>
                  <ExternalLink aria-hidden />
                  {t('viewer.openOutside')}
                </Button>
              )}
            </DialogActions>
          </>
        )}
      </DialogPopup>
    </Dialog>
  )
}

/** What is shown of one file. */
function Sight({ entry }: { entry: FileEntry }) {
  switch (sightOf(entry)) {
    case 'image':
      return <Picture entry={entry} />
    case 'video':
      return <Clip entry={entry} />
    case 'text':
      return <Text entry={entry} />
    case 'pdf':
      return <Document entry={entry} />
    case 'sound':
    case 'other':
      return <Known entry={entry} />
  }
}

/** The line under what is shown: what the file is, in numbers. */
function Facts({ entry, extra }: { entry: FileEntry; extra?: string }) {
  return (
    <p className="shrink-0 font-mono text-xs text-faint">
      {[
        extra,
        formatBytes(entry.size),
        entry.modified === undefined ? undefined : formatMoment(entry.modified),
      ]
        .filter((one) => one !== undefined)
        .join(' · ')}
    </p>
  )
}

/**
 * A picture: fitted to the window when it opens, and enlarged from there -
 * by the buttons or the wheel - up to four times its own size, scrolled
 * inside its frame.
 */
function Picture({ entry }: { entry: FileEntry }) {
  const { t } = useTranslation()
  // `null` is "fitted to the frame"; a number is a share of its own size.
  const [zoom, setZoom] = useState<number | null>(null)
  const [size, setSize] = useState<{ width: number; height: number } | null>(null)
  const frame = useRef<HTMLDivElement>(null)

  /** The share of its own size the picture is drawn at while fitted. */
  const fitted = () => {
    const box = frame.current
    if (box === null || size === null) return 1
    return Math.min(1, box.clientWidth / size.width, box.clientHeight / size.height)
  }
  const step = (direction: 1 | -1) => {
    const now = zoom ?? fitted()
    const next =
      direction === 1
        ? ZOOMS.find((one) => one > now + 0.001)
        : [...ZOOMS].reverse().find((one) => one < now - 0.001)
    if (next !== undefined) setZoom(next)
  }

  return (
    <>
      <div
        ref={frame}
        className={cn(
          'flex min-h-0 flex-1 rounded-md bg-bg',
          zoom === null ? 'items-center justify-center overflow-hidden ring-in' : 'overflow-auto',
        )}
        onWheel={(event) => {
          if (!event.ctrlKey) return
          step(event.deltaY < 0 ? 1 : -1)
        }}
      >
        <img
          src={fileSrc(entry.path)}
          alt={entry.name}
          draggable={false}
          onLoad={(event) =>
            setSize({
              width: event.currentTarget.naturalWidth,
              height: event.currentTarget.naturalHeight,
            })
          }
          style={
            zoom === null || size === null
              ? undefined
              : { width: size.width * zoom, height: size.height * zoom }
          }
          className={cn(
            zoom === null ? 'max-h-full max-w-full object-contain' : 'm-auto max-w-none shrink-0',
          )}
        />
      </div>
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        <Facts
          entry={entry}
          extra={size === null ? undefined : `${String(size.width)}×${String(size.height)}`}
        />
        <span className="ml-auto flex items-center gap-1">
          <Button
            variant="icon"
            size="icon-sm"
            aria-label={t('viewer.zoomOut')}
            title={t('viewer.zoomOut')}
            onClick={() => step(-1)}
          >
            <Minus aria-hidden />
          </Button>
          <span className="w-12 text-center font-mono text-xs text-dim tabular-nums">
            {zoom === null ? t('viewer.fit') : `${String(Math.round(zoom * 100))}%`}
          </span>
          <Button
            variant="icon"
            size="icon-sm"
            aria-label={t('viewer.zoomIn')}
            title={t('viewer.zoomIn')}
            onClick={() => step(1)}
          >
            <Plus aria-hidden />
          </Button>
          <Button size="sm" disabled={zoom === null} onClick={() => setZoom(null)}>
            {t('viewer.fit')}
          </Button>
          <Button size="sm" disabled={zoom === 1} onClick={() => setZoom(1)}>
            {t('viewer.actual')}
          </Button>
        </span>
      </div>
    </>
  )
}

/** A clip, with its sound, its own controls - seeking, the whole screen -
 *  and how fast it plays. */
function Clip({ entry }: { entry: FileEntry }) {
  const { t } = useTranslation()
  const [speed, setSpeed] = useState<string>('1')
  const video = useRef<HTMLVideoElement>(null)
  useEffect(() => {
    if (video.current !== null) video.current.playbackRate = Number(speed)
  }, [speed])

  return (
    <>
      <div className="flex min-h-0 flex-1 items-center justify-center rounded-md bg-bg">
        <video
          ref={video}
          src={fileSrc(entry.path)}
          controls
          aria-label={entry.name}
          className="max-h-full max-w-full"
        />
      </div>
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        <Facts entry={entry} />
        <span className="ml-auto flex items-center gap-2 text-xs text-dim">
          {t('viewer.speed')}
          <Select
            className="w-24"
            aria-label={t('viewer.speed')}
            value={speed}
            onChange={setSpeed}
            options={SPEEDS.map((one) => ({ value: one, label: `${one}×` }))}
          />
        </span>
      </div>
    </>
  )
}

/** A text: a lyric, subtitles, notes - read through the backend, which
 *  reads only so much of a file and only where kilna looks. */
function Text({ entry }: { entry: FileEntry }) {
  const { t } = useTranslation()
  const text = useQuery({
    queryKey: ['assets', 'text', entry.path, entry.modified ?? ''],
    queryFn: () => readTextFile(entry.path),
  })

  if (text.isPending) return <Skeleton className="min-h-0 w-full flex-1" />
  if (text.isError) {
    return (
      <>
        <p role="alert" className="text-sm text-bad">
          {humanError(text.error)}
        </p>
        <Known entry={entry} />
      </>
    )
  }
  return (
    <>
      <pre className="selectable min-h-0 flex-1 overflow-auto rounded-md border border-line bg-bg p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap">
        {text.data.text}
      </pre>
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        <Facts entry={entry} extra={text.data.truncated ? t('viewer.truncated') : undefined} />
        <CopyButton
          className="ml-auto"
          value={text.data.text}
          label={t('viewer.copyText')}
          copiedLabel={t('viewer.textCopied')}
        />
      </div>
    </>
  )
}

/** A document, drawn by the window's own reader where it has one. */
function Document({ entry }: { entry: FileEntry }) {
  return (
    <>
      <iframe
        src={fileSrc(entry.path)}
        title={entry.name}
        className="min-h-0 w-full flex-1 rounded-md border border-line bg-bg"
      />
      <Facts entry={entry} />
    </>
  )
}

/** What is known of a file the window cannot draw: its name, its kind, its
 *  size, when it changed and where it lies. */
function Known({ entry }: { entry: FileEntry }) {
  const { t } = useTranslation()
  const rows: [string, string][] = [
    [t('files.column.name'), entry.name],
    [t('files.column.type'), t(`files.category.${entry.category}`)],
    [t('files.column.size'), formatBytes(entry.size)],
    ...(entry.modified === undefined
      ? []
      : ([[t('files.column.modified'), formatMoment(entry.modified)]] as [string, string][])),
    [t('viewer.path'), entry.path],
  ]
  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3">
      <p className="text-sm text-dim">{t('viewer.notDrawn')}</p>
      <dl className="grid max-w-full grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1 text-sm">
        {rows.map(([label, value]) => (
          <div key={label} className="contents">
            <dt className="text-faint">{label}</dt>
            <dd className="selectable min-w-0 break-all">{value}</dd>
          </div>
        ))}
      </dl>
    </div>
  )
}
