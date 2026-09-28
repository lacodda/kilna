import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import type { ProfileConfig, Workspace } from '@/lib/api/types'
import { updateProfileConfig } from '@/lib/api/workspace'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { ActionBar, ActionBarButton, ActionBarSpacer } from '@/components/ui/action-bar'
import { Button } from '@/components/ui/button'
import { StatusDot } from '@/components/ui/status-dot'
import type { ProfileDraftState } from '@/features/settings/useProfileDraft'

/**
 * "There are unsaved changes", with Save and Discard - at the foot of the
 * Profile section while its draft differs from what is stored.
 *
 * Until v0.79 the Save button was the last thing on a form several screens
 * long, and nothing said the profile had unsaved edits anywhere above it. The
 * bar stands at the pane's foot instead, in sight from any field, and it is
 * there only while there is something to save - so its appearing is the news.
 */
export function ProfileDraftBar({ draft }: { draft: ProfileDraftState }) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const profile = useProfile()

  const save = useAppMutation({
    mutationFn: (config: ProfileConfig) => updateProfileConfig(profile.id, config),
    // The vocabulary is on every screen: labels, statuses, kinds, axes.
    refresh: [keys.workspace, keys.profiles, keys.catalogue],
    failure: 'toast.profileSaveFailed',
    onSuccess: (saved) => {
      // What is stored is what the draft is measured against. Put the saved
      // profile in its place now rather than when the refetch lands, or the
      // bar goes on saying there are unsaved changes for a round trip after
      // they were saved. The draft lets go by itself once it matches - and
      // anything typed while the save was on its way stays a draft.
      client.setQueryData<Workspace>(keys.workspace, (current) =>
        current?.profile?.id === saved.id ? { ...current, profile: saved } : current,
      )
    },
  })

  // No question before: a discard is taken back from the toast, as a
  // deletion is - one click after the fact rather than one every time.
  const discard = () => {
    const kept = draft.discard()
    if (kept !== null) {
      say.undoable(t('settings.discarded'), t('toast.undo'), () => draft.restore(kept))
    }
  }

  return (
    <ActionBar aria-label={t('settings.unsaved')} className="flex-1">
      <StatusDot status="warn" size="sm" label={t('settings.unsaved')} showLabel />
      <ActionBarSpacer />
      <ActionBarButton
        render={<Button variant="ghost" size="sm" />}
        disabled={save.isPending}
        onClick={discard}
      >
        {t('settings.discard')}
      </ActionBarButton>
      <ActionBarButton
        render={<Button variant="primary" size="sm" />}
        disabled={save.isPending}
        onClick={() => save.mutate(draft.config)}
      >
        {t('editor.save')}
      </ActionBarButton>
    </ActionBar>
  )
}
