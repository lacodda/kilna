import { createContext, useContext } from 'react'

/*
 * Judging blind: the past verdict is held back until this card has one of its
 * own.
 *
 * It lived in the Score tab's own state, so it hid the list and the trail
 * there - and the card's header, standing right above the tab, went on showing
 * "tier · total" of the last score, which is the one number the mode exists to
 * hide. The card holds it now, and everything on the card that would say the
 * past verdict reads it from here.
 */
export interface BlindJudging {
  /** Judging blind was switched on for this card. */
  blind: boolean
  /** The verdict was given (or asked for), so what was held back is shown. */
  revealed: boolean
  setBlind: (blind: boolean) => void
  setRevealed: (revealed: boolean) => void
}

export const BlindJudgingContext = createContext<BlindJudging>({
  blind: false,
  revealed: false,
  setBlind: () => {},
  setRevealed: () => {},
})

export function useBlindJudging(): BlindJudging & { hiding: boolean } {
  const judging = useContext(BlindJudgingContext)
  return { ...judging, hiding: judging.blind && !judging.revealed }
}
