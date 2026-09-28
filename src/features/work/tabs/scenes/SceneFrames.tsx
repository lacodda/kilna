import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { Check, ImagePlus, Maximize2, Trash2 } from 'lucide-react'
import { fileSrc } from '@/lib/api/assets'
import {
  attachSceneFrame,
  clearSceneFrame,
  detachSceneFrame,
  pasteSceneFrame,
  reorderSceneFrames,
  selectSceneFrame,
} from '@/lib/api/scenes'
import type { SceneFrame } from '@/lib/api/types'
import i18n from '@/i18n'
import { landsOn } from '@/lib/drop'
import { CLIPS, PICTURES } from '@/lib/media'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { movedTo, VIDEO } from '@/lib/scenes'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { ReorderGrip, ReorderIndicator, useReorder } from '@/components/ui/reorderable-list'

interface Props {
  workId: string
  sceneId: string
  /** The scene's number, for the sentences that name it. */
  number: number
  /** Which of the scene's material this strip shows: `frame` or `video`. */
  kind: string
  /** The scene's material OF THIS KIND, already filtered by the caller. */
  frames: SceneFrame[]
  /** Open the viewer on this frame. */
  onOpen: (frame: SceneFrame) => void
}

/**
 * The pictures drawn for one scene: a list of candidates, one of them chosen.
 *
 * Three ways in, because a picture arrives three ways and each is the cheapest
 * for its case. A file on disk comes through the picker. A file dragged from a
 * folder comes through the window's drop event. And a picture looked at in the
 * generator's own browser tab comes through Ctrl+V — the paste event hands over
 * the bytes with no permission and no round trip through the disk, which is why
 * the plan calls it the cheapest path.
 *
 * Both events belong to the window, not to this element, and every open
 * scene has two strips listening - so each strip decides for itself whether an
 * event is its own, or one picture became a copy in every open scene and both
 * of its strips. A drop is the strip's when it lands on it: the event carries
 * the pointer's position. A paste has no position, so it is the scene the
 * person is working in - the one holding the focus, or the only one open.
 *
 * The candidates stand in a column since v0.81, each on a row with its grip,
 * because their order is the person's to set - the one to try first on top -
 * and the line's way to reorder a list is a grip on a row: the pointer drags
 * it, Alt with an arrow moves it from the keyboard.
 */
export function SceneFrames({ workId, sceneId, number, kind, frames, onOpen }: Props) {
  const { t } = useTranslation()
  // One component for both strips rather than a copy of it for clips: they
  // differ in the words, the file extensions and the element that draws a
  // thumbnail, and in nothing else - the same three doors in, the same
  // choosing, the same removal, the same order.
  const isVideo = kind === VIDEO
  const word = (key: string, values?: Record<string, unknown>) =>
    t(isVideo ? `scenes.video.${key}` : `scenes.${key}`, values ?? {})
  const [over, setOver] = useState(false)
  const region = useRef<HTMLDivElement>(null)

  // What every gesture on this strip changes: its own frames.
  const REFRESHED = [keys.sceneFramesFor(workId)] as const

  const attach = useAppMutation({
    mutationFn: (source: string) => attachSceneFrame(sceneId, kind, source),
    refresh: REFRESHED,
    onSuccess: () => say.ok(word('frameAdded', { number })),
  })

  const paste = useAppMutation({
    mutationFn: ({ bytes, name }: { bytes: Uint8Array; name: string }) =>
      pasteSceneFrame(sceneId, kind, bytes, name),
    refresh: REFRESHED,
    onSuccess: () => say.ok(word('framePasted', { number })),
  })

  const choose = useAppMutation({
    mutationFn: (id: string) => selectSceneFrame(id),
    refresh: REFRESHED,
  })

  const unchoose = useAppMutation({
    mutationFn: () => clearSceneFrame(sceneId, kind),
    refresh: REFRESHED,
  })

  const remove = useAppMutation({
    mutationFn: (id: string) => detachSceneFrame(id),
    refresh: REFRESHED,
  })

  // The whole order of this kind travels, as the board's does: the backend
  // checks it names every picture of the scene once, and a list built from
  // the one that moved would be refused for the rest - rightly.
  const reorder = useAppMutation({
    mutationFn: (ids: string[]) => reorderSceneFrames(sceneId, kind, ids),
    refresh: REFRESHED,
  })

  const ids = frames.map((frame) => frame.id)
  const order = useReorder({
    order: ids,
    onMove: (id, to) => reorder.mutate(movedTo(ids, id, to)),
    disabled: reorder.isPending,
  })

  const pick = async () => {
    const chosen = await open({
      multiple: false,
      filters: [{ name: word('frames'), extensions: isVideo ? CLIPS : PICTURES }],
    })
    if (typeof chosen === 'string') attach.mutate(chosen)
  }

  // Dropping a file on the window. The event carries paths, so the file is
  // already on disk and goes the same way as the picker's - to this strip only
  // when the pointer is over it.
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
          for (const source of payload.paths) attach.mutate(source)
        }
      })
      .then((unlisten) => {
        if (alive) stop = unlisten
        else unlisten()
      })
    return () => {
      alive = false
      stop?.()
      setOver(false)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sceneId])

  // Pasting a picture. The clipboard hands over bytes rather than a path, and
  // the bytes are what travels: writing the file here would mean giving the
  // window filesystem permissions for one temporary write, when the backend
  // already owns the directory the picture is going to.
  useEffect(() => {
    // Only the stills. Ctrl+V exists here because a picture is LOOKED AT in
    // the generator's own tab and copied from it; a clip is downloaded and
    // then dragged, so a paste listener on the video strip would only ever
    // catch a picture dropped into the wrong list.
    if (isVideo) return
    const onPaste = async (event: ClipboardEvent) => {
      const items = event.clipboardData?.files
      if (!items || items.length === 0) return
      if (!pastesHere(region.current)) return
      const file = items[0]
      if (!file || !file.type.startsWith('image/')) {
        say.failed(t('scenes.frameNotAnImage'))
        return
      }
      event.preventDefault()
      try {
        const extension = file.type.split('/')[1] ?? 'png'
        const name = file.name || `pasted-${Date.now()}.${extension}`
        const bytes = new Uint8Array(await file.arrayBuffer())
        paste.mutate({ bytes, name })
      } catch (error) {
        say.failed(error)
      }
    }
    window.addEventListener('paste', onPaste)
    return () => window.removeEventListener('paste', onPaste)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sceneId, number, isVideo])

  return (
    <div
      ref={region}
      data-scene-strip={kind}
      className={cn(
        'flex flex-col gap-2 rounded-md border border-dashed p-2 transition-colors',
        over ? 'border-accent bg-accent-soft' : 'border-line',
      )}
    >
      <div className="flex items-center gap-2">
        <span className="caption">{word('frames')}</span>
        <Button size="sm" variant="ghost" onClick={() => void pick()} disabled={attach.isPending}>
          <ImagePlus aria-hidden />
          {word('addFrame')}
        </Button>
        {frames.some((frame) => frame.is_selected) && (
          <Button size="sm" variant="ghost" onClick={() => unchoose.mutate()}>
            {word('unchooseFrame')}
          </Button>
        )}
      </div>

      {frames.length === 0 ? (
        <p className="text-xs text-dim">{over ? word('dropFrame') : word('addFrameHint')}</p>
      ) : (
        // The line where a picture would land is drawn against this box, so
        // it holds the list rather than standing inside it.
        <div {...order.listProps} className="relative">
          <ul className="flex flex-col gap-1">
            {frames.map((frame) => (
              <li
                key={frame.id}
                {...order.rowProps(frame.id)}
                className={cn(
                  'flex items-center gap-2',
                  order.dragging === frame.id && 'opacity-50',
                )}
              >
                {/* A grip only where there is somewhere to move to. */}
                {frames.length > 1 ? (
                  <ReorderGrip {...order.gripProps(frame.id)} title={word('moveFrame')} />
                ) : (
                  <span aria-hidden className="w-3.5 shrink-0" />
                )}
                {/* eslint-disable-next-line dowel/no-raw-button -- a thumbnail: the picture is the control and its border is the chosen state; no primitive draws a pressable picture */}
                <button
                  type="button"
                  onClick={() => onOpen(frame)}
                  className={cn(
                    'block shrink-0 overflow-hidden rounded border bg-soft',
                    frame.is_selected ? 'border-good ring-1 ring-good' : 'border-line',
                  )}
                  title={frame.original_name ?? word('openFrame')}
                >
                  {/* Contained, never cropped: a frame may be wide or tall and
                      a common crop would misrepresent one of them. */}
                  {isVideo ? (
                    // Muted, and not preloaded past its first frame: a list
                    // of four clips that each fetched themselves whole would
                    // spend the board's memory on pictures of their opening
                    // second, which is all that is shown here.
                    <video
                      src={fileSrc(frame.path)}
                      muted
                      preload="metadata"
                      className="size-20 object-contain"
                    />
                  ) : (
                    <img
                      src={fileSrc(frame.path)}
                      alt={frame.original_name ?? ''}
                      className="size-20 object-contain"
                    />
                  )}
                </button>
                <span className="min-w-0 flex-1 truncate text-xs text-dim">
                  {frame.original_name ?? ''}
                </span>

                {/* Always there, at the end of the row: on the strip of
                    v0.69 they appeared over the thumbnail under the pointer
                    and were hard to hit - the owner called them cramped. */}
                <Button
                  variant="icon"
                  size="icon-sm"
                  onClick={() => choose.mutate(frame.id)}
                  title={frame.is_selected ? word('chosenFrame') : word('chooseFrame')}
                  aria-label={frame.is_selected ? word('chosenFrame') : word('chooseFrame')}
                  aria-pressed={frame.is_selected}
                  className={cn(frame.is_selected && 'text-good')}
                >
                  <Check aria-hidden />
                </Button>
                <Button
                  variant="icon"
                  size="icon-sm"
                  onClick={() => onOpen(frame)}
                  title={word('openFrame')}
                  aria-label={word('openFrame')}
                >
                  <Maximize2 aria-hidden />
                </Button>
                <Button
                  variant="icon"
                  size="icon-sm"
                  onClick={() => remove.mutate(frame.id)}
                  title={word('removeFrame')}
                  aria-label={word('removeFrame')}
                  className="hover:text-bad"
                >
                  <Trash2 aria-hidden />
                </Button>
              </li>
            ))}
          </ul>
          <ReorderIndicator offset={order.slotOffset} />
        </div>
      )}
    </div>
  )
}

/**
 * Whether a paste belongs to the scene this strip is in.
 *
 * A paste carries no position, so it goes to the open scene the person is
 * working in: the one whose open row holds the focus, or - when the focus is
 * in none of them - the only one open. With several open and the focus in
 * none, nothing is guessed: the first of them says where to click, once.
 */
function pastesHere(strip: HTMLElement | null): boolean {
  const own = strip?.closest('[data-scene-detail]')
  if (own === null || own === undefined) return false
  const open = Array.from(document.querySelectorAll('[data-scene-detail]'))
  const focused = open.find((detail) => detail.contains(document.activeElement))
  if (focused !== undefined) return focused === own
  if (open.length === 1) return true
  if (open[0] === own) say.warn(i18n.t('scenes.pasteWhere'))
  return false
}
