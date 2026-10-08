import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import type { Asset } from '@/lib/api/types'
import {
  attachPicture,
  insertAt,
  isPicturePath,
  pastePicture,
  pictureLine,
} from '@/lib/notePictures'
import { say } from '@/lib/toast'
import { cn } from '@/lib/utils'
import { Textarea, type TextareaProps } from '@/components/ui/textarea'

/**
 * A note's body being written, with pictures (v0.93): a picture pasted from
 * the clipboard, or a picture file dropped on the window while the body is
 * open, is copied into the workspace and written into the text where the
 * caret stood - `![](media/<id>.png)`, which the reading view draws.
 *
 * The note's own text box rather than a second editor: the notes and a card
 * of the canon each had a textarea, and both get the same pictures from the
 * same place. See `lib/notePictures` for the shape the links take.
 */
export function NoteBodyEditor({
  noteId,
  value,
  onText,
  className,
  ...props
}: Omit<TextareaProps, 'value' | 'onChange'> & {
  noteId: string
  value: string
  /** The text, changed - by typing, or by a picture arriving in it. */
  onText: (text: string) => void
}) {
  const { t } = useTranslation()
  const box = useRef<HTMLTextAreaElement>(null)
  const [adding, setAdding] = useState(0)
  // The text as it stands, for a picture that arrives after more was typed.
  const latest = useRef(value)
  useLayoutEffect(() => {
    latest.current = value
  }, [value])

  /** Write pictures into the text where the caret was when they were asked
   *  for, one after another, each on a line of its own. */
  const arrive = (pictures: Promise<Asset>[], at: number) => {
    if (pictures.length === 0) return
    setAdding((count) => count + 1)
    Promise.allSettled(pictures)
      .then((results) => {
        let text = latest.current
        let caret = at
        for (const result of results) {
          if (result.status === 'rejected') {
            say.failedTo(t('notes.pictureFailed'), result.reason)
            continue
          }
          const line = pictureLine(result.value)
          const next = insertAt(text, caret, line)
          caret += next.length - text.length
          text = next
        }
        if (text !== latest.current) onText(text)
      })
      .finally(() => setAdding((count) => count - 1))
  }

  // Files dropped on the window while the body is open. The event belongs to
  // the whole window, as on the Files tab, so this listens only while the
  // editor is on screen; a file that is not a picture is left alone.
  const arriving = useRef(arrive)
  useLayoutEffect(() => {
    arriving.current = arrive
  })
  useEffect(() => {
    const subscription = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type !== 'drop') return
      const pictures = payload.paths.filter(isPicturePath)
      if (pictures.length === 0) return
      const at = box.current?.selectionStart ?? latest.current.length
      arriving.current(
        pictures.map((path) => attachPicture(noteId, path)),
        at,
      )
    })
    return () => {
      void subscription.then((unlisten) => {
        unlisten()
      })
    }
  }, [noteId])

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col">
      <Textarea
        {...props}
        ref={box}
        value={value}
        onChange={(event) => onText(event.target.value)}
        onPaste={(event) => {
          // A picture on the clipboard - a screenshot, a picture copied from
          // a page or a file copied in the file manager - goes into the
          // text; anything else is pasted the way text is.
          const pictures = [...event.clipboardData.files].filter((file) =>
            file.type.startsWith('image/'),
          )
          if (pictures.length === 0) return
          event.preventDefault()
          arrive(
            pictures.map((file) => pastePicture(noteId, file)),
            event.currentTarget.selectionStart,
          )
        }}
        className={cn('min-h-0 flex-1', className)}
      />
      <p aria-live="polite" className="shrink-0 px-5 py-1 text-xs text-faint">
        {adding > 0 ? t('notes.pictureAdding') : t('notes.pictureHint')}
      </p>
    </div>
  )
}
