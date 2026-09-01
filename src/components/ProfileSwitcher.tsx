import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { activateProfile, listProfiles } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { Select } from '@/components/ui/AppSelect'

interface Props {
  activeId: string
  onSwitched: () => void
}

// Several profiles live in one workspace; switching changes the vocabulary
// every screen speaks in, and which works are visible.
export function ProfileSwitcher({ activeId, onSwitched }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()

  const profiles = useQuery({ queryKey: keys.profiles, queryFn: listProfiles })

  const activate = useMutation({
    mutationFn: activateProfile,
    onSuccess: (_result, id) => {
      // Every screen is scoped to the active profile, so nothing cached
      // survives the switch.
      void client.invalidateQueries()

      const name = profiles.data?.find((profile) => profile.id === id)?.name
      if (name !== undefined) say.ok(t('toast.profileSwitched', { name }))
      onSwitched()
    },
    onError: (cause) => say.failed(cause),
  })

  if (profiles.data === undefined || profiles.data.length < 2) return null

  return (
    <Select
      className="w-full"
      aria-label={t('status.profile')}
      value={activeId}
      onChange={(id) => {
        if (id !== activeId) activate.mutate(id)
      }}
      options={profiles.data.map((profile) => ({ value: profile.id, label: profile.name }))}
    />
  )
}
