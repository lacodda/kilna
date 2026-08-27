import { createContext, useContext } from 'react'

/**
 * How anything on screen opens the assistant.
 *
 * The drawer lives in the launcher, but what opens it does not: a banner about
 * a waiting question, a toast about a finished task, a link from a card. Rather
 * than each of them reaching into the launcher, they ask for this.
 *
 * `null` outside the provider, which only happens in isolated tests — callers
 * treat it as "no drawer here" rather than crashing a screen over it.
 */
export interface Assistant {
  /** Open the drawer, on `chatId` when one is named. */
  open: (chatId?: string) => void
}

export const AssistantContext = createContext<Assistant | null>(null)

export function useAssistant(): Assistant {
  return useContext(AssistantContext) ?? { open: () => undefined }
}
