import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { activateProfile, listProfiles, type Profile } from '@/lib/api'
import { Select } from '@/components/ui/Select'

interface Props {
  activeId: string
  onSwitched: () => void
}

// Several profiles live in one workspace; switching changes the vocabulary
// every screen speaks in, and which works are visible.
export function ProfileSwitcher({ activeId, onSwitched }: Props) {
  const { t } = useTranslation()
  const [profiles, setProfiles] = useState<Profile[]>([])
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    listProfiles()
      .then(setProfiles)
      .catch((cause: unknown) => setError(String(cause)))
  }, [activeId])

  if (error !== null) {
    return (
      <span role="alert" className="text-xs text-red-600">
        {error}
      </span>
    )
  }

  if (profiles.length < 2) return null

  return (
    <Select
      className="w-40"
      aria-label={t('status.profile')}
      value={activeId}
      onChange={(id) => {
        if (id === activeId) return
        activateProfile(id)
          .then(onSwitched)
          .catch((cause: unknown) => setError(String(cause)))
      }}
      options={profiles.map((profile) => ({ value: profile.id, label: profile.name }))}
    />
  )
}
