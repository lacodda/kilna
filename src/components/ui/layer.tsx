import { createContext, useContext, useEffect, useRef, type ReactNode, type RefObject } from 'react'
import { POPUP_FLOORS, type Rung } from '@/lib/layers'

/*
 * Which layer a popup opened inside an overlay lands on.
 *
 * The stacking scale is a flat ladder - `--z-menu: 30`, `--z-floating: 40`,
 * `--z-modal: 60` - and every overlay portals to the document body, so the
 * ladder alone decides who covers whom. It answers the common question right
 * and has nothing to say about the one that bit us: a date popover opened from
 * inside a dialog. Both are body-level siblings, the popover a 40 and the
 * dialog a 60, so the month drew UNDER the dialog that asked for it - what the
 * pilot photographed on the release editor.
 *
 * Two other fixes were measured first, and both are worth writing down because
 * both look right until the screen is looked at.
 *
 * Portalling the popover INTO the dialog puts it in the dialog's own stacking
 * context, where document order settles it, and it does paint above the dialog
 * - hit-tested, with the negative control failing as it should. But the dialog
 * popup carries `overflow-y: auto` (a dialog taller than the window has to
 * scroll, v0.44), and a scroll box clips an absolutely-positioned popup inside
 * it: the month came out with one row of days showing. A popup anchored near
 * the edge of its container has to be allowed to leave it, which is why
 * overlays portal to the body at all.
 *
 * Raising the popover's own number works exactly once: whatever number it is
 * given, it is then above EVERY dialog including the ones it is not inside,
 * and the next pairing arrives with the same bug and no number left for it.
 *
 * So the popup stays at the body where nothing clips it, and it is the LAYER
 * that travels. An overlay opens a host of its own next to itself on the body
 * and redefines `--z-floating` inside it; a popup opened anywhere within that
 * overlay portals into the host and reads the raised value through ordinary
 * CSS inheritance - `popover.tsx` and `preview-card.tsx` already say
 * `[z-index:var(--z-floating)]`, so neither has to be edited or even know this
 * exists. Nothing is clipped, nothing outranks a dialog it is not inside, and
 * an overlay inside an overlay keeps climbing without anyone inventing a
 * number at a call site.
 */

const LayerContext = createContext<RefObject<HTMLElement | null> | null>(null)

/** Where a popup opened here should portal to, or `undefined` when there is no
 * overlay above it and the body is right. Pass it straight to a popup's
 * `container`. */
export function usePopupContainer(): RefObject<HTMLElement | null> | undefined {
  return useContext(LayerContext) ?? undefined
}

/** Opens a raised host beside this overlay and offers it to whatever opens
 * inside. The host sits on the body, so a popup portalled into it is anchored
 * and clipped by nothing; what it inherits is the floor. */
export function LayerProvider({ rung, children }: { rung: Rung; children: ReactNode }) {
  // The node is made during the first render rather than in the effect. A
  // popup's `container` is read as it mounts, and a host that only appeared on
  // a second pass would let the first paint go to the body - the one frame in
  // which the bug is still there. It is a ref and not state for the same
  // reason: nothing renders because of it, so nothing should re-render for it.
  const host = useRef<HTMLElement | null>(null)
  host.current ??= document.createElement('div')

  useEffect(() => {
    const node = host.current
    if (node === null) return
    // The host draws nothing - it is a place on the body and a raised floor.
    // It stays `position: static`, so Base UI still measures the popup against
    // the viewport and the trigger, exactly as it would at the body itself.
    //
    // Every rung a popup might read is raised, not `--z-floating` alone. A
    // popover reads floating, a menu and a select read `--z-menu` - and for as
    // long as only one of them was lifted, the select in a dialog and the ±
    // menu on the full-screen stage stayed on the page's own floor and drew
    // under the overlay that opened them. Which variable a component happens
    // to read is not something its caller should have to know.
    const floor = String(POPUP_FLOORS[rung])
    node.style.setProperty('--z-floating', floor)
    node.style.setProperty('--z-menu', floor)
    node.style.setProperty('--z-popup', floor)
    document.body.append(node)
    return () => node.remove()
  }, [rung])

  return <LayerContext.Provider value={host}>{children}</LayerContext.Provider>
}
