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
        'absolute right-1.5 top-1.5 cursor-pointer rounded-[7px] border border-line bg-raise px-1.5 py-0.5 text-[11px] text-dim',
        'opacity-0 transition-opacity hover:text-text focus-visible:opacity-100 group-hover:opacity-100',
      )
      button.addEventListener('click', () => {
        void navigator.clipboard.writeText(code)
        // A tick the reader sees where they clicked; words would need i18n a
        // DOM string cannot follow when the language switches mid-session.
        button.textContent = '✓'
        setTimeout(() => {
          button.textContent = copyLabel
        }, 1200)
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
        'text-sm leading-relaxed',
        // Tailwind's preflight strips list markers and heading sizes, so the
        // few tags markdown actually produces are styled back by hand rather
        // than by pulling in a typography plugin for one screen.
        '[&_h1]:mt-4 [&_h1]:mb-2 [&_h1]:text-lg [&_h1]:font-semibold',
        '[&_h2]:mt-4 [&_h2]:mb-2 [&_h2]:text-base [&_h2]:font-semibold',
        '[&_h3]:mt-3 [&_h3]:mb-1.5 [&_h3]:text-sm [&_h3]:font-semibold',
        '[&_p]:my-2 [&_ul]:my-2 [&_ol]:my-2',
        '[&_ul]:list-disc [&_ol]:list-decimal [&_ul]:pl-5 [&_ol]:pl-5',
        '[&_li]:my-0.5',
        '[&_blockquote]:my-2 [&_blockquote]:border-l-2 [&_blockquote]:border-line-2 [&_blockquote]:pl-3 [&_blockquote]:text-dim',
        '[&_code]:rounded [&_code]:bg-soft [&_code]:px-1 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-[0.85em]',
        '[&_pre]:my-2 [&_pre]:overflow-x-auto [&_pre]:rounded-[9px] [&_pre]:bg-soft [&_pre]:p-3',
        '[&_pre_code]:bg-transparent [&_pre_code]:p-0',
        '[&_a]:text-accent-2 [&_a]:underline',
        '[&_strong]:font-semibold [&_em]:italic',
        '[&_hr]:my-4 [&_hr]:border-line',
        '[&>*:first-child]:mt-0 [&>*:last-child]:mb-0',
        className,
      )}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  )
}
