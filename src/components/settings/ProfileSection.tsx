import { ProfileEditor } from '@/components/ProfileEditor'
import { StatusDrift } from '@/components/StatusDrift'

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
