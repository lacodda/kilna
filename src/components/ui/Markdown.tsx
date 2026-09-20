import { useEffect, useMemo, useRef } from 'react'
import DOMPurify from 'dompurify'
import { marked } from 'marked'
import { cn } from '@/lib/utils'

/**
 * Rendered markdown.
 *
 * Everything is sanitised before it reaches the DOM. The text here is the
 * user's own writing and, since v0.28, assistant replies — and this app can
 * reach the file system, so unsanitised HTML from a model's output is not a
 * risk worth taking twice.
 *
 * Rendering is synchronous on purpose: `marked` can return a promise when
 * extensions are registered, and an editor preview that arrives a frame late
 * flickers on every keystroke.
 */
export function Markdown({
  body,
  className,
  copyLabel,
}: {
  body: string
  className?: string
  /** When set, every code block gets a button with this label that copies it. */
  copyLabel?: string
}) {
  const container = useRef<HTMLDivElement>(null)

  const html = useMemo(() => {
    const parsed = marked.parse(body, { async: false, breaks: true })
    return DOMPurify.sanitize(parsed)
  }, [body])

  // Copy buttons are injected after render rather than through the markdown
  // pipeline: the sanitiser would strip them, and rightly so — they are ours,
  // not the text's. The code is captured before the button joins the block,
  // so what is copied is exactly what was written.
  useEffect(() => {
    if (copyLabel === undefined) return
    const root = container.current
    if (root === null) return

    const buttons: HTMLButtonElement[] = []
    for (const block of root.querySelectorAll('pre')) {
      const code = (block.textContent ?? '').replace(/\n$/, '')
      const button = document.createElement('button')
      button.type = 'button'
      button.textContent = copyLabel
      button.className = cn(
        'absolute right-1.5 top-1.5 cursor-pointer rounded-md border border-line bg-raise px-1.5 py-0.5 text-[11px] text-dim',
        'opacity-0 transition-opacity hover:text-text focus-visible:opacity-100 group-hover:opacity-100',
      )
      button.addEventListener('click', () => {
        // The tick only after the clipboard confirms — a tick on a failed
        // copy is worse than none. It is a symbol, not a word: a DOM string
        // cannot follow the language when it switches mid-session.
        navigator.clipboard.writeText(code).then(
          () => {
            button.textContent = '✓'
            setTimeout(() => {
              button.textContent = copyLabel
            }, 1200)
          },
          () => {},
        )
      })
      block.classList.add('group', 'relative')
      block.appendChild(button)
      buttons.push(button)
    }

    return () => {
      for (const button of buttons) button.remove()
    }
  }, [html, copyLabel])

  return (
    <div
      ref={container}
      className={cn(
        /*
         * `prose-tight` from dowel, which is where these rules live now.
         *
         * They were twenty lines of `[&_h1]:…` arbitrary variants here - the
         * shape a product is forced into, because rendered markdown arrives as
         * HTML nobody authored and there is no element to put a class on. Only
         * a descendant selector reaches those tags, and a className string is
         * the only place a component can write one.
         *
         * The tight variant rather than the full one: this is text inside a
         * panel that already has its own width, so the reading measure is the
         * container's. The rhythm matches what was drawn here before.
         */
        'prose prose-tight',
        // Selection is handed back where the shell switched it off. The
        // stylesheet does this too; kept because `selectable` is this app's
        // own word for it and other screens are checked against it.
        'selectable',
        className,
      )}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  )
}
