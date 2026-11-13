import type { ReactNode } from 'react'
import { ScrollArea } from '@/components/ui/scroll-area'

/**
 * The scroller a version's text sits in: the line's overlay bar, and a
 * content box at least as tall as the window onto it.
 *
 * The frame's `Scroll` hands its content the height the content asks for,
 * which is right for a list and wrong for a text: a short text being edited
 * ended a line under its last word, and a click in the empty half of the
 * panel below it went nowhere instead of putting the caret at the end. Here
 * the content box is a column with the viewport's height as its floor, so
 * the text - or the box it is typed into - can take the whole panel and
 * still grow past it and scroll.
 *
 * Held to the vertical axis, as the frame's `Scroll` is: a text wraps to the
 * column rather than pushing it wider.
 */
export function TextScroll({ label, children }: { label: string; children: ReactNode }) {
  return (
    <ScrollArea
      label={label}
      orientation="vertical"
      className="flex-1"
      viewportClassName="*:flex! *:min-h-full! *:flex-col!"
    >
      {children}
    </ScrollArea>
  )
}
