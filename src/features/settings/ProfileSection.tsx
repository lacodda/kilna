import { Divider } from '@/components/ui/divider'
import { ProfileEditor } from '@/features/settings/ProfileEditor'
import { ShippedOffers } from '@/features/settings/ShippedOffers'
import { StatusDrift } from '@/features/settings/StatusDrift'

/** The craft's vocabulary, what kilna ships now beside it, and the check
 * that the statuses still match the facts. */
export function ProfileSection() {
  return (
    <div className="flex flex-col gap-4">
      <ShippedOffers except="prompts" />
      <ProfileEditor />
      <Divider />
      <StatusDrift />
    </div>
  )
}
