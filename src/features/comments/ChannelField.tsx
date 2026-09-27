import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

interface Props {
  value: string
  onChange: (value: string) => void
  /** The channels already used, most waiting first. */
  known: string[]
  autoFocus?: boolean
}

/**
 * Where a comment was written: a word the person types, with the words
 * already used one click away.
 *
 * Free text on purpose (migration 0027): a channel is wherever the work went
 * out, and a list someone has to maintain before the first comment can be
 * kept is a list nobody maintains. The chips are what stops the second
 * comment from being filed under a near-miss of the first one's channel.
 */
export function ChannelField({ value, onChange, known, autoFocus }: Props) {
  const { t } = useTranslation()
  const others = known.filter((channel) => channel !== value.trim()).slice(0, 8)

  return (
    <div className="flex flex-col gap-1.5">
      <Input
        autoFocus={autoFocus}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={t('comments.channelPlaceholder')}
        aria-label={t('comments.channel')}
      />
      {others.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {others.map((channel) => (
            <button
              key={channel}
              type="button"
              onClick={() => onChange(channel)}
              className={cn(
                'cursor-pointer rounded-full border border-line px-2 py-0.5 text-[11px] text-dim transition-colors hover:border-line-2 hover:text-text',
              )}
            >
              {channel}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
