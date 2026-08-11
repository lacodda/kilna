import { createContext, useContext } from 'react'
import type { Profile } from '@/lib/api'

// The active profile is the vocabulary every screen speaks in, so it is read
// once and shared rather than fetched per component.
export const ProfileContext = createContext<Profile | null>(null)

export function useProfile(): Profile {
  const profile = useContext(ProfileContext)
  if (profile === null) {
    throw new Error('useProfile used outside a loaded workspace')
  }
  return profile
}

// Label for a key from one of the profile's vocabularies, falling back to the
// raw key so an unknown value is visible rather than blank.
export function labelOf(kinds: { key: string; label: string }[], key: string): string {
  return kinds.find((kind) => kind.key === key)?.label ?? key
}
