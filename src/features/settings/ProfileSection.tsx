import { Divider } from '@/components/ui/divider'
import { ProfileEditor } from '@/features/settings/ProfileEditor'
import { StatusDrift } from '@/features/settings/StatusDrift'

/** The craft's vocabulary, and the check that the statuses still match the facts. */
export function ProfileSection() {
  return (
    <div className="flex flex-col gap-4">
      <ProfileEditor />
      <Divider />
      <StatusDrift />
    </div>
  )
}
