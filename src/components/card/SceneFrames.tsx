import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { Check, ImagePlus, Maximize2, Trash2 } from 'lucide-react'
import {
  attachSceneFrame,
  clearSceneFrame,
  detachSceneFrame,
  fileSrc,
  pasteSceneFrame,
  selectSceneFrame,
  type SceneFrame,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { VIDEO } from '@/lib/scenes'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

/** The picture formats a generator gives back, and the picker offers. */
export const PICTURES = ['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif']

/** The clip formats an animator gives back. */
export const VIDEOS = ['mp4', 'webm', 'mov', 'm4v']

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
 * The pictures drawn for one scene: a strip of candidates, one of them chosen.
 *
 * Three ways in, because a picture arrives three ways and each is the cheapest
 * for its case. A file on disk comes through the picker. A file dragged from a
 * folder comes through the window's drop event. And a picture looked at in the
 * generator's own browser tab comes through Ctrl+V — the paste event hands over
 * the bytes with no permission and no round trip through the disk, which is why
 * the plan calls it the cheapest path.
 *
 * The drop and paste listeners live only while this scene's frames are on
 * screen. Both events belong to the window, not to this element, and a picture
 * dropped while another scene was open must not land in this one — the lesson
 * the Files tab wrote down in v0.67.
 */
export function SceneFrames({ workId, sceneId, number, kind, frames, onOpen }: Props) {
  const { t } = useTranslation()
  // One component for both strips rather than a copy of it for clips: they
  // differ in the words, the file extensions and the element that draws a
  // thumbnail, and in nothing else - the same three doors in, the same
  // choosing, the same removal.
  const isVideo = kind === VIDEO
  const word = (key: string, values?: Record<string, unknown>) =>
    t(isVideo ? `scenes.video.${key}` : `scenes.${key}`, values ?? {})
  const client = useQueryClient()
  const [over, setOver] = useState(false)
  const region = useRef<HTMLDivElement>(null)

  const refresh = () => {
    void client.invalidateQueries({ queryKey: keys.sceneFramesFor(workId) })
    void client.invalidateQueries({ queryKey: keys.journal })
  }

  const attach = useMutation({
    mutationFn: (source: string) => attachSceneFrame(sceneId, kind, source),
    onSuccess: () => {
      refresh()
      say.ok(word('frameAdded', { number }))
    },
    onError: (error: unknown) => say.failed(String(error)),
  })

  const paste = useMutation({
    mutationFn: ({ bytes, name }: { bytes: Uint8Array; name: string }) =>
      pasteSceneFrame(sceneId, kind, bytes, name),
    onSuccess: () => {
      refresh()
      say.ok(word('framePasted', { number }))
    },
    onError: (error: unknown) => say.failed(String(error)),
  })

  const choose = useMutation({
    mutationFn: (id: string) => selectSceneFrame(id),
    onSuccess: refresh,
    onError: (error: unknown) => say.failed(String(error)),
  })

  const unchoose = useMutation({
    mutationFn: () => clearSceneFrame(sceneId, kind),
    onSuccess: refresh,
    onError: (error: unknown) => say.failed(String(error)),
  })

  const remove = useMutation({
    mutationFn: (id: string) => detachSceneFrame(id),
    onSuccess: refresh,
    onError: (error: unknown) => say.failed(String(error)),
  })

  const pick = async () => {
    const chosen = await open({
      multiple: false,
      filters: [{ name: word('frames'), extensions: isVideo ? VIDEOS : PICTURES }],
    })
    if (typeof chosen === 'string') attach.mutate(chosen)
  }

  // Dropping a file on the window. The event carries paths, so the file is
  // already on disk and goes the same way as the picker's.
  useEffect(() => {
    let stop: (() => void) | undefined
    let alive = true
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload
        if (payload.type === 'over') {
          setOver(true)
          return
        }
        setOver(false)
        if (payload.type === 'drop') {
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
        say.failed(String(error))
      }
    }
    window.addEventListener('paste', onPaste)
    return () => window.removeEventListener('paste', onPaste)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sceneId, number, isVideo])

  return (
    <div
      ref={region}
      className={cn(
        'rounded-md border border-dashed p-2 transition-colors',
        over ? 'border-accent bg-accent/5' : 'border-line',
      )}
    >
      <div className="mb-2 flex items-center gap-2">
        <span className="text-xs font-medium text-dim">{word('frames')}</span>
        <Button size="sm" variant="ghost" onClick={() => void pick()} disabled={attach.isPending}>
          <ImagePlus className="size-3.5" aria-hidden />
          {word('addFrame')}
        </Button>
        {frames.some((frame) => frame.is_selected) && (
          <Button size="sm" variant="ghost" onClick={() => unchoose.mutate()}>
            {word('unchooseFrame')}
          </Button>
        )}
      </div>

      {frames.length === 0 ? (
        <p className="text-xs text-dim">
          {over ? word('dropFrame') : word('addFrameHint')}
        </p>
      ) : (
        <ul className="flex flex-wrap gap-2">
          {frames.map((frame) => (
            <li key={frame.id} className="group relative">
              <button
                type="button"
                onClick={() => onOpen(frame)}
                className={cn(
                  'block overflow-hidden rounded border bg-surface-2',
                  frame.is_selected ? 'border-good ring-1 ring-good' : 'border-line',
                )}
                title={frame.original_name ?? word('openFrame')}
              >
                {/* Contained, never cropped: a frame may be wide or tall and
                    a common crop would misrepresent one of them. */}
                {isVideo ? (
                  // Muted, and not preloaded past its first frame: a strip of
                  // four clips that each fetched themselves whole would spend
                  // the board's memory on pictures of their opening second,
                  // which is all that is shown here.
                  <video
                    src={fileSrc(frame.path)}
                    muted
                    preload="metadata"
                    className="size-24 bg-surface-2 object-contain"
                  />
                ) : (
                  <img
                    src={fileSrc(frame.path)}
                    alt={frame.original_name ?? ''}
                    className="size-24 object-contain"
                  />
                )}
              </button>

              {/* Buttons, not bare elements with a quarter of a line of
                  padding: these were ~14px targets on a strip that appears
                  under the pointer, which is a hard thing to hit and the
                  reason the owner called them cramped. `icon-sm` is the
                  line's smallest real control. */}
              <div className="absolute inset-x-0 bottom-0 flex justify-between gap-0.5 bg-surface/80 px-1 py-0.5 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
                <Button
                  variant="icon"
                  size="icon-sm"
                  onClick={() => choose.mutate(frame.id)}
                  title={frame.is_selected ? word('chosenFrame') : word('chooseFrame')}
                  aria-label={frame.is_selected ? word('chosenFrame') : word('chooseFrame')}
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
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
