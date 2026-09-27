import { ProfileEditor } from '@/features/settings/ProfileEditor'
import { StatusDrift } from '@/features/settings/StatusDrift'

/** The craft's vocabulary, and the check that the statuses still match the facts. */
export function ProfileSection() {
  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <ProfileEditor />
      <hr className="border-line" />
      <StatusDrift />
    </div>
  )
}
