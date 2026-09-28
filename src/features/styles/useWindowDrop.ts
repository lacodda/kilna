import { useEffect, useRef, useState, type RefObject } from 'react'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { landsOn } from '@/lib/drop'

/**
 * Files dropped from a folder onto one element of the screen: whether a drag
 * is over it now, and the paths when they are let go on it.
 *
 * The drop is the window's, not the element's - Tauri takes it before the
 * page does - so every target on the screen hears every drop and keeps the
 * ones that land on its own box. The styles screen has two: the card that
 * makes a style from what is dropped, and the open style, which takes it as
 * references. One picture must not become a copy in both.
 */
export function useWindowDrop(
  target: RefObject<HTMLElement | null>,
  onDrop: (paths: string[]) => void,
): boolean {
  const [over, setOver] = useState(false)
  // The latest handler, read when a drop lands: subscribing again on every
  // render would drop the events that arrive while the old listener is being
  // taken down.
  const handler = useRef(onDrop)
  useEffect(() => {
    handler.current = onDrop
  }, [onDrop])

  useEffect(() => {
    let stop: (() => void) | undefined
    let alive = true
    const here = (position: { x: number; y: number }) =>
      landsOn(target.current?.getBoundingClientRect(), position, window.devicePixelRatio)
    void getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === 'leave') {
          setOver(false)
          return
        }
        if (payload.type === 'enter' || payload.type === 'over') {
          setOver(here(payload.position))
          return
        }
        setOver(false)
        if (here(payload.position)) handler.current(payload.paths)
      })
      .then((unlisten) => {
        if (alive) stop = unlisten
        else unlisten()
      })
    return () => {
      alive = false
      stop?.()
    }
  }, [target])

  return over
}
