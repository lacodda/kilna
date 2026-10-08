import { useEffect, useMemo, useRef } from 'react'
import { createPortal } from 'react-dom'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import DOMPurify from 'dompurify'
import { marked } from 'marked'
import type { ResolvedLink } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { BODY_PREFIX, urlOf } from '@/lib/notePictures'
import { hrefOf, replaceWikiLinks, wikiLinks } from '@/lib/wikilink'
import { cn } from '@/lib/utils'
import { CopyButton } from '@/components/ui/copy-button'

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
  plainLinks = false,
  onToggleTask,
}: {
  body: string
  className?: string
  /** When set, every code block gets a copy button named this. */
  copyLabel?: string
  /** Leave `[[work:id]]` as written rather than resolving it. For a box
      being typed into, where a link turning into a title mid-sentence would
      move the text under the caret. */
  plainLinks?: boolean
  /** Makes `- [ ]` boxes live: a click hands back which box, counted in
      order, and the caller rewrites the body (see `lib/checklist`). Without
      it the boxes are drawn and cannot be ticked, which is right for text
      that is not the reader's to change — an assistant's answer, a version
      under a score. */
  onToggleTask?: (index: number) => void
}) {
  const { t } = useTranslation()
  const container = useRef<HTMLDivElement>(null)
  const navigate = useNavigate()

  const links = useMemo(() => (plainLinks ? [] : wikiLinks(body)), [body, plainLinks])
  // Keyed by the ids the body names rather than by the body: two notes citing
  // the same work share one answer, and editing the prose around a link does
  // not refetch it.
  const named = useMemo(() => {
    const works = [...new Set(links.filter((l) => l.target === 'work').map((l) => l.id))].sort()
    const versions = [
      ...new Set(links.filter((l) => l.target === 'version').map((l) => l.id)),
    ].sort()
    return { works, versions }
  }, [links])

  const resolved = useQuery({
    ...queries.resolvedLinks(named.works, named.versions),
    enabled: links.length > 0,
  })

  // A picture written into a note names the file it was stored as,
  // `media/<id>.png` (v0.93, ADR 0058): read against the workspace's own
  // folder, which is asked for only by a body that shows one.
  const showsPictures = body.includes(`](${BODY_PREFIX}`)
  const media = useQuery({ ...queries.mediaDirectory(), enabled: showsPictures })

  const html = useMemo(() => {
    const found = new Map<string, ResolvedLink>(
      (resolved.data ?? []).map((one) => [`${one.target}:${one.id}`, one]),
    )
    // Rewritten before `marked` sees it, into ordinary markdown links, so the
    // sanitiser and the prose styles treat them as the links they are. An
    // unresolved one becomes its own text: a link to something deleted, or one
    // whose answer has not arrived yet, reads as the words rather than as a
    // dead anchor that goes nowhere when clicked.
    const source =
      links.length === 0
        ? body
        : replaceWikiLinks(body, (link) => {
            const target = found.get(`${link.target}:${link.id}`)
            if (target === undefined) return link.label ?? link.id
            const href = hrefOf(link, () => target.workId)
            const text = (link.label ?? target.title).replace(/[[\]]/g, '')
            return href === undefined ? text : `[${text}](${href})`
          })
    const parsed = marked.parse(source, { async: false, breaks: true })
    const clean = DOMPurify.sanitize(parsed)
    // After the sanitiser, which keeps a relative `src` as it is: only a bare
    // name inside `media/` becomes the workspace's file, and anything else
    // stays the link it was written as.
    const directory = media.data
    if (directory === undefined || !clean.includes(`src="${BODY_PREFIX}`)) return clean
    return clean.replace(/src="(media\/[^"]*)"/g, (whole, link: string) => {
      const url = urlOf(directory, link)
      return url === null ? whole : `src="${url}"`
    })
  }, [body, links, resolved.data, media.data])

  // Copy buttons stay out of the markdown pipeline: the sanitiser would strip
  // them, and rightly so — they are ours, not the text's. Each code block gets
  // a host of its own, made with the HTML it belongs to, and the button is
  // portalled into it; the effect below puts each host into its block. The
  // code is read from the HTML, so what is copied is exactly what was written.
  const codeBlocks = useMemo(() => {
    if (copyLabel === undefined) return []
    const parsed = new DOMParser().parseFromString(html, 'text/html')
    return [...parsed.querySelectorAll('pre')].map((block) => {
      const host = document.createElement('span')
      host.className = 'absolute top-1.5 right-1.5'
      return { code: (block.textContent ?? '').replace(/\n$/, ''), host }
    })
  }, [html, copyLabel])

  useEffect(() => {
    const root = container.current
    if (root === null || codeBlocks.length === 0) return

    const blocks = root.querySelectorAll('pre')
    for (const [index, { host }] of codeBlocks.entries()) {
      const block = blocks[index]
      if (block === undefined) continue
      block.classList.add('group', 'relative')
      block.appendChild(host)
    }

    return () => {
      for (const { host } of codeBlocks) host.remove()
    }
  }, [codeBlocks])

  // The boxes are markdown's, so they are found after render, the way the
  // code blocks are. The click is prevented rather than let through: the box
  // shows what the body says, and it changes when the body does - a box that
  // ticked itself ahead of a write that then failed would be lying.
  useEffect(() => {
    const root = container.current
    if (root === null || onToggleTask === undefined) return

    const boxes = [...root.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')]
    const cleanups = boxes.map((box, index) => {
      box.disabled = false
      box.classList.add('cursor-pointer')
      const onClick = (event: MouseEvent) => {
        event.preventDefault()
        onToggleTask(index)
      }
      box.addEventListener('click', onClick)
      return () => box.removeEventListener('click', onClick)
    })

    return () => {
      for (const cleanup of cleanups) cleanup()
    }
  }, [html, onToggleTask])

  // An internal link is the router's, not the browser's: letting the anchor
  // navigate would reload the whole window and lose every open panel. The
  // listener sits on the container because the anchors are markdown's, and
  // there is no element of ours to put an `onClick` on.
  useEffect(() => {
    const root = container.current
    if (root === null) return

    const onClick = (event: MouseEvent) => {
      // A modified click is the person asking for the browser's behaviour.
      if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.shiftKey) return
      const anchor = (event.target as Element | null)?.closest('a')
      const href = anchor?.getAttribute('href')
      if (href === undefined || href === null || !href.startsWith('/')) return
      event.preventDefault()
      void navigate(href)
    }

    root.addEventListener('click', onClick)
    return () => root.removeEventListener('click', onClick)
  }, [navigate])

  return (
    <>
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
          // A task item draws its box instead of a bullet, and a done one reads
          // as done. Only a descendant selector reaches markdown's own tags.
          '[&_li:has(>input[type=checkbox])]:list-none [&_li:has(>input[type=checkbox])]:-ml-5',
          '[&_li:has(>input:checked)]:text-faint [&_li:has(>input:checked)]:line-through',
          '[&_li>input[type=checkbox]]:mr-1.5 [&_li>input[type=checkbox]]:accent-accent',
          className,
        )}
        dangerouslySetInnerHTML={{ __html: html }}
      />
      {codeBlocks.map(({ code, host }, index) =>
        createPortal(
          // The tick only after the clipboard confirms — a tick on a failed
          // copy is worse than none.
          <CopyButton value={code} label={copyLabel ?? ''} copiedLabel={t('assistant.copied')} />,
          host,
          String(index),
        ),
      )}
    </>
  )
}
