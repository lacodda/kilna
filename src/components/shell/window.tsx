import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { cn } from '@/lib/utils'

/*
 * The window's own frame, for a window that has no system frame.
 *
 * With `decorations: false` the system draws nothing, so everything it used
 * to do is the page's: dragging the window by its title bar, double-click to
 * maximise, the three buttons, and the edges you grab to resize. Each is
 * small; the reason to take them on at all is that a system title bar over an
 * application title bar costs a strip of every laptop screen for nothing, and
 * scheda already made the same trade.
 */

/** Whether the window is maximised, kept current as the window changes.
 *
 *  The window can be maximised without our buttons - a drag to the top edge,
 *  the keyboard, a snap layout - so the answer follows the window rather than
 *  our own last click. */
function useMaximized(): boolean {
  const [maximized, setMaximized] = useState(false)

  useEffect(() => {
    const window = getCurrentWindow()
    const read = () => {
      window.isMaximized().then(setMaximized).catch(() => undefined)
    }
    read()
    const unlisten = window.onResized(read)
    return () => {
      unlisten.then((stop) => stop()).catch(() => undefined)
    }
  }, [])

  return maximized
}

const BUTTON =
  'flex h-full w-[46px] cursor-default items-center justify-center text-dim transition-colors hover:bg-soft hover:text-text'

/** The window controls, in the order Windows puts them. */
export function WindowButtons() {
  const { t } = useTranslation()
  const maximized = useMaximized()

  return (
    <div className="flex h-full shrink-0 items-stretch">
      <button
        type="button"
        className={BUTTON}
        aria-label={t('shell.minimize')}
        title={t('shell.minimize')}
        onClick={() => void getCurrentWindow().minimize()}
      >
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
          <path d="M0 5h10" stroke="currentColor" strokeWidth="1" fill="none" />
        </svg>
      </button>
      <button
        type="button"
        className={BUTTON}
        aria-label={t(maximized ? 'shell.restore' : 'shell.maximize')}
        title={t(maximized ? 'shell.restore' : 'shell.maximize')}
        onClick={() => void getCurrentWindow().toggleMaximize()}
      >
        {maximized ? (
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
            <path
              d="M2.5 2.5V0.5h7v7h-2M0.5 2.5h7v7h-7z"
              stroke="currentColor"
              strokeWidth="1"
              fill="none"
            />
          </svg>
        ) : (
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
            <rect
              x="0.5"
              y="0.5"
              width="9"
              height="9"
              stroke="currentColor"
              strokeWidth="1"
              fill="none"
            />
          </svg>
        )}
      </button>
      <button
        type="button"
        className={cn(BUTTON, 'hover:bg-bad hover:text-on-bad')}
        aria-label={t('shell.close')}
        title={t('shell.close')}
        onClick={() => void getCurrentWindow().close()}
      >
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
          <path d="M0 0l10 10M10 0L0 10" stroke="currentColor" strokeWidth="1" fill="none" />
        </svg>
      </button>
    </div>
  )
}

/** Makes an element behave like a title bar: drag to move, double-click to
 *  maximise. Both are what the system used to do for free.
 *
 *  Two handlers rather than one. A `pointerdown` cannot recognise a double
 *  click: its `detail` counts clicks of the *mouse* event sequence, and the
 *  second press still arrives as 1. So the press starts a drag, and
 *  `dblclick`, which the browser is the one qualified to detect, maximises.
 *
 *  Dragging starts on the first movement, not on the press. `startDragging`
 *  hands the window over to the system - which is what keeps snap layouts and
 *  drag-to-edge working - but from that moment the webview stops seeing the
 *  mouse. Calling it on `pointerdown` ate the second click of every double
 *  click, and maximising never happened. */
export function useTitleBarGestures() {
  const shouldHandle = (target: EventTarget | null) =>
    // A press on a control has already been handled by the control.
    !(target as HTMLElement | null)?.closest(
      'button, a, input, [role="menu"], [role="menuitem"], [role="dialog"]',
    )

  const onPointerDown = useCallback((event: React.PointerEvent) => {
    if (event.button !== 0 || !shouldHandle(event.target)) return

    const start = { x: event.clientX, y: event.clientY }
    const THRESHOLD = 4

    const onMove = (move: PointerEvent) => {
      if (
        Math.abs(move.clientX - start.x) < THRESHOLD &&
        Math.abs(move.clientY - start.y) < THRESHOLD
      ) {
        return
      }
      stop()
      void getCurrentWindow().startDragging()
    }
    const stop = () => {
      window.removeEventListener('pointermove', onMove)
      window.removeEventListener('pointerup', stop)
      window.removeEventListener('pointercancel', stop)
    }

    window.addEventListener('pointermove', onMove)
    window.addEventListener('pointerup', stop)
    window.addEventListener('pointercancel', stop)
  }, [])

  const onDoubleClick = useCallback((event: React.MouseEvent) => {
    if (event.button !== 0 || !shouldHandle(event.target)) return
    void getCurrentWindow().toggleMaximize()
  }, [])

  return { onPointerDown, onDoubleClick }
}

/** The eight edges and corners a frameless window still has to offer. */
const RESIZE_HANDLES = [
  'North',
  'South',
  'East',
  'West',
  'NorthEast',
  'NorthWest',
  'SouthEast',
  'SouthWest',
] as const

const EDGE = 5
const CORNER = 10

/** Where each strip sits and which cursor it shows. Inline styles rather than
 *  classes: eight positions of a few pixels each are geometry, not design. */
const EDGE_STYLE: Record<(typeof RESIZE_HANDLES)[number], React.CSSProperties> = {
  North: { top: 0, left: CORNER, right: CORNER, height: EDGE, cursor: 'ns-resize' },
  South: { bottom: 0, left: CORNER, right: CORNER, height: EDGE, cursor: 'ns-resize' },
  East: { top: CORNER, bottom: CORNER, right: 0, width: EDGE, cursor: 'ew-resize' },
  West: { top: CORNER, bottom: CORNER, left: 0, width: EDGE, cursor: 'ew-resize' },
  NorthEast: { top: 0, right: 0, width: CORNER, height: CORNER, cursor: 'nesw-resize' },
  NorthWest: { top: 0, left: 0, width: CORNER, height: CORNER, cursor: 'nwse-resize' },
  SouthEast: { bottom: 0, right: 0, width: CORNER, height: CORNER, cursor: 'nwse-resize' },
  SouthWest: { bottom: 0, left: 0, width: CORNER, height: CORNER, cursor: 'nesw-resize' },
}

/** Invisible strips along the window's edges.
 *
 *  A frameless window has no border to grab, so these put one back. They sit
 *  outside the flow, above everything, and are only a few pixels wide -
 *  enough to hit, not enough to steal a click meant for the text. A maximised
 *  window has no edges to drag, and leaving the strips in place would mean
 *  the top few pixels of the title bar stop taking clicks. */
export function ResizeEdges() {
  const maximized = useMaximized()
  if (maximized) return null

  return (
    <>
      {RESIZE_HANDLES.map((direction) => (
        <div
          key={direction}
          aria-hidden
          className="fixed z-[100]"
          style={EDGE_STYLE[direction]}
          onPointerDown={(event) => {
            if (event.button !== 0) return
            event.preventDefault()
            void getCurrentWindow().startResizeDragging(direction)
          }}
        />
      ))}
    </>
  )
}
