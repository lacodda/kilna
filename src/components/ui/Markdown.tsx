import { useMemo } from 'react'
import DOMPurify from 'dompurify'
import { marked } from 'marked'
import { cn } from '@/lib/utils'

/**
 * Rendered markdown.
 *
 * Everything is sanitised before it reaches the DOM. The text here is the
 * user's own writing today, but the same component renders assistant replies
 * from v0.26 — and this app can reach the file system, so unsanitised HTML from
 * a model's output is not a risk worth taking twice.
 *
 * Rendering is synchronous on purpose: `marked` can return a promise when
 * extensions are registered, and an editor preview that arrives a frame late
 * flickers on every keystroke.
 */
export function Markdown({ body, className }: { body: string; className?: string }) {
  const html = useMemo(() => {
    const parsed = marked.parse(body, { async: false, breaks: true })
    return DOMPurify.sanitize(parsed)
  }, [body])

  return (
    <div
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
