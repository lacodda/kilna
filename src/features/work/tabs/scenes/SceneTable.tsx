import { useRef, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Panel } from '@/components/ui/panel'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Table, TableBody, TableEmpty, TableHead, TableHeader } from '@/components/ui/table'
import { cn } from '@/lib/utils'
import { CELL } from '@/features/work/tabs/scenes/SceneRow'

interface Props {
  /** The kind names kinds of shot, so the board has a column for them. */
  shots: boolean
  /** The board has an About column. */
  about: boolean
  /** What the body says when the filters leave no row; null while they
      leave some. */
  none: string | null
  /** The rows, given the width of the pane an open row must fit. */
  children: (paneWidth: number | null) => ReactNode
}

/** How many columns a row spans, for the open row under it. */
export function columnsOf({ shots, about }: { shots: boolean; about: boolean }): number {
  return 6 + (shots ? 1 : 0) + (about ? 1 : 0)
}

/**
 * The board as one table: fifty scenes read as a list of rows, not as fifty
 * stacked cards you scroll past to compare two timings.
 *
 * In a panel of its own that takes the height the tab leaves it and scrolls
 * inside, the way the catalogue's table does: the scroller is the table's
 * own, so the headings have something to stick to and stay named while fifty
 * rows move under them, and the sideways bar sits at the foot of the panel
 * rather than under the last scene.
 *
 * The columns are as wide as what they hold, and the description takes the
 * rest. There was a minimum of 1152px under a fixed layout until v0.81,
 * because every cell was a boxed field and the column of buttons at the end
 * was eight wide; with the cells undressed and the buttons in one menu the
 * board fits a laptop, and it scrolls sideways only when the window is
 * narrower than the cells themselves.
 */
export function SceneTable({ shots, about, none, children }: Props) {
  const { t } = useTranslation()

  // How wide the pane actually is. The table may be wider than it on a
  // narrow window, and an open row must stay inside what the eye can see
  // rather than inheriting the table's width.
  //
  // A callback ref rather than an effect, which fires the moment the node
  // arrives and again when it leaves; and `clientWidth`, not the observer's
  // `contentRect`: the viewport is the box that scrolls and its content is
  // the table, which may be wider. What an open row must fit inside is the
  // part a person can see.
  const [paneWidth, setPaneWidth] = useState<number | null>(null)
  const watching = useRef<ResizeObserver | null>(null)
  const pane = (box: HTMLDivElement | null) => {
    watching.current?.disconnect()
    watching.current = null
    if (box === null) return
    setPaneWidth(box.clientWidth)
    const watch = new ResizeObserver(() => setPaneWidth(box.clientWidth))
    watch.observe(box)
    watching.current = watch
  }

  const columns = columnsOf({ shots, about })
  const heading = cn(CELL, 'whitespace-nowrap')

  return (
    // Compact, as the mockup's board is: a row of fields at the height of a
    // row of text, so a laptop shows a dozen scenes rather than eight.
    <Panel data-density="compact" className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <ScrollArea label={t('card.tab.scenes')} className="flex-1" viewportRef={pane}>
        <Table>
          {/* On the panel's ground, which is what the rows slide under, and
              above the rows - a sticky heading is a layer of its own. dowel's
              `z-sticky` names a variable Tailwind makes no class of, so the
              height is said here, as the catalogue says it. */}
          <TableHead sticky className="z-3 bg-raise">
            <tr className="border-b border-line caption">
              <TableHeader className={cn(heading, 'pr-1 text-right')}>
                {t('scenes.number')}
              </TableHeader>
              <TableHeader className={heading}>{t('scenes.section')}</TableHeader>
              <TableHeader className={heading}>{t('scenes.span')}</TableHeader>
              {shots && <TableHeader className={heading}>{t('scenes.shotType')}</TableHeader>}
              {/* The column that takes what the others leave. */}
              <TableHeader className={cn(heading, 'w-full')}>{t('scenes.description')}</TableHeader>
              {about && <TableHeader className={heading}>{t('scenes.about')}</TableHeader>}
              <TableHeader className={heading}>{t('scenes.readiness')}</TableHeader>
              <TableHeader className="px-1">
                <span className="sr-only">{t('scenes.rowActions')}</span>
              </TableHeader>
            </tr>
          </TableHead>
          <TableBody>
            {/* The headings stay above a filter that matched nothing: the
                table is the furniture, only its rows are missing. */}
            {none === null ? (
              children(paneWidth)
            ) : (
              <TableEmpty colSpan={columns}>{none}</TableEmpty>
            )}
          </TableBody>
        </Table>
      </ScrollArea>
    </Panel>
  )
}
