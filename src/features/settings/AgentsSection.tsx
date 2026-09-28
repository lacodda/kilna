import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { queries } from '@/lib/query/queries'
import { say } from '@/lib/toast'
import { CopyButton } from '@/components/ui/copy-button'

/** Agents outside the window: the MCP door, and how to register it. */
export function AgentsSection() {
  const { t } = useTranslation()
  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('data.mcp')}</h3>
        <p className="text-sm text-dim">{t('data.mcpHint')}</p>
        <McpRegistration />
        <p className="text-xs text-dim">{t('data.mcpAfter')}</p>
      </section>
    </div>
  )
}

/**
 * The command that registers this build with Claude Code, ready to copy.
 *
 * Shown rather than run: kilna does not know which shell, which agent or
 * whether the person wants it at all. The path is this executable's own,
 * so it is right for the build in front of them and wrong for none.
 */
function McpRegistration() {
  const { t } = useTranslation()
  const command = useQuery(queries.mcpRegistration())

  if (command.data === undefined) return null

  // `group` so the copy button shows as soon as the pointer is over the
  // command, not only once it finds the button.
  return (
    <div className="group flex items-center gap-2">
      <code className="selectable min-w-0 flex-1 overflow-x-auto rounded-md border border-line bg-soft px-2.5 py-1.5 font-mono text-xs whitespace-nowrap">
        {command.data}
      </code>
      {/* The tick only once the clipboard said yes; a refusal is said rather
          than swallowed, or the person pastes whatever was there. */}
      <CopyButton
        value={command.data}
        label={t('data.mcpCopy')}
        copiedLabel={t('data.mcpCopied')}
        onCopy={(ok) => {
          if (!ok) say.failed(t('work.copyFailed'))
        }}
        title={t('data.mcpCopy')}
      />
    </div>
  )
}
