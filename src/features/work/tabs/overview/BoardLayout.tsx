import type { ReactNode } from 'react'
import type { OverviewLayout, WidgetSize } from '@/lib/api/types'
import { leadOf, type Placed } from '@/lib/overview'
import { cn } from '@/lib/utils'
import { Panel } from '@/components/ui/panel'
import { WidgetLook, type Presentation } from '@/features/work/tabs/overview/Widget'

interface Props {
  layout: OverviewLayout
  /** The widgets this work draws, in order - what does not apply already gone. */
  widgets: readonly Placed[]
  /** One widget, drawn; it reads how from `WidgetLook`. */
  render: (widget: Placed) => ReactNode
}

/**
 * The five arrangements of one set of widgets, from the owner's mockup
 * (`#L1`-`#L5`): an even grid, a lead column with a rail, bands across the
 * board, a mosaic sized by weight, and a sheet read as one document.
 *
 * Every layout draws the same widgets; what differs is where each stands, how
 * much room it gets, and whether it is a card or a row (`WidgetLook`). The
 * narrow cases are the board's own width rather than the window's
 * (`@container` on the scrolling box): the board sits inside the card, beside
 * the rail, and a window wide enough for two columns can still hold a board
 * that is not. `data-layout` says which arrangement is drawn, for a test.
 */
export function BoardLayout({ layout, widgets, render }: Props) {
  const draw = (widget: Placed, presentation: Presentation, place?: string, size?: WidgetSize) => (
    <WidgetLook key={widget.id} value={{ presentation, size: size ?? widget.size, place }}>
      {render(widget)}
    </WidgetLook>
  )

  switch (layout) {
    case 'grid':
      // Every widget one cell, and every cell as wide as the next: the even
      // grid reads a board as a set of equal facts. A row is as tall as its
      // tallest widget, and none is shorter than the mockup's card.
      return (
        <div
          data-layout="grid"
          className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] items-start gap-2.5"
        >
          {widgets.map((widget) => draw(widget, 'card', 'min-h-33', 's'))}
        </div>
      )

    case 'lead':
      return <Lead widgets={widgets} draw={draw} />

    case 'bands':
      // One band across the board per widget: a caption, the fact, and what
      // acts on it, read top to bottom like a report.
      return (
        <div data-layout="bands" className="flex flex-col gap-2.5">
          {widgets.map((widget) => draw(widget, 'band'))}
        </div>
      )

    case 'mosaic':
      // Sizes by weight: two by two for the text, two across for what reads
      // in a line, one cell for a figure. Dense, so a figure fills the hole
      // a wide widget leaves beside it.
      return (
        <div
          data-layout="mosaic"
          className="grid grid-flow-row-dense auto-rows-[minmax(112px,auto)] grid-cols-4 gap-2.5 @max-[65rem]:grid-cols-2 @max-[38rem]:grid-cols-1"
        >
          {widgets.map((widget) => draw(widget, 'card', SPAN[widget.size]))}
        </div>
      )

    case 'sheet':
      // One document: a line per widget in a single panel, captions down the
      // left, divided by hairlines rather than boxed one by one.
      return (
        <Panel data-layout="sheet" className="overflow-hidden">
          {widgets.map((widget) => draw(widget, 'line'))}
        </Panel>
      )
  }
}

/** The cells a widget spans in the mosaic; one column narrow, it spans one. */
const SPAN: Record<WidgetSize, string> = {
  s: '',
  m: 'col-span-2 @max-[38rem]:col-span-1',
  l: 'col-span-2 row-span-2 @max-[38rem]:col-span-1 @max-[38rem]:row-span-1',
}

/**
 * The owner's layout (24.09): what is read in a lead column, what is glanced
 * at on a rail beside it (`leadOf`). The lead is the wider, 1.65 to 1, as the
 * mockup has it; narrow, the rail goes under the lead. A board with nothing
 * for one of the two columns draws the other across the whole width rather
 * than beside an empty one.
 */
function Lead({
  widgets,
  draw,
}: {
  widgets: readonly Placed[]
  draw: (widget: Placed, presentation: Presentation, place?: string) => ReactNode
}) {
  const { lead, rail } = leadOf(widgets)
  const both = lead.length > 0 && rail.length > 0

  return (
    <div
      data-layout="lead"
      className={cn(
        'grid items-start gap-2.5',
        both && '@min-[40rem]:grid-cols-[minmax(0,1.65fr)_minmax(0,1fr)]',
      )}
    >
      {lead.length > 0 && (
        <div className="flex min-w-0 flex-col gap-2.5">
          {lead.map((row) =>
            row.kind === 'one' ? (
              // The text takes the lead's height as the mockup draws it: a
              // reading surface, not a two-line teaser.
              draw(row.widget, 'card', row.widget.size === 'l' ? 'min-h-62.5' : undefined)
            ) : (
              // Each of the pair as tall as it has to be: a widget does not
              // stretch over empty space to match its neighbour (the widget
              // catalogue's rule), so a one-line style prompt beside twelve
              // fields stays one line tall.
              <div
                key={`${row.widgets[0].id}+${row.widgets[1].id}`}
                className="grid min-w-0 grid-cols-2 items-start gap-2.5 @max-[30rem]:grid-cols-1"
              >
                {draw(row.widgets[0], 'card')}
                {draw(row.widgets[1], 'card')}
              </div>
            ),
          )}
        </div>
      )}
      {rail.length > 0 && (
        <div className="flex min-w-0 flex-col gap-2.5">
          {rail.map((widget) => draw(widget, 'card'))}
        </div>
      )}
    </div>
  )
}
