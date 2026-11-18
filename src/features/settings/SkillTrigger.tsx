import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { save } from '@tauri-apps/plugin-dialog'
import type { Client, TriggerFile } from '@/lib/api/types'
import { installSkillTrigger } from '@/lib/api/workspace'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { ConfirmAction } from '@/components/ConfirmAction'
import { Button } from '@/components/ui/button'
import { CopyButton } from '@/components/ui/copy-button'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'

/**
 * The thin trigger an agent is given (FR-17): a short file in the agent's
 * own place that sends it to kilna's MCP server whenever the person talks
 * about their works - and not a line of a method in it, so it never goes
 * stale and never carries the craft out of the space. Shown whole before it
 * is written, copied or installed; a file already there is replaced only on
 * a yes.
 */
export function SkillTrigger() {
  const { t } = useTranslation()
  const files = useQuery(queries.skillTriggers()).data ?? []
  const [client, setClient] = useState<Client>('claude-code')
  const [asking, setAsking] = useState(false)
  const file = files.find((one) => one.client === client)

  const install = useAppMutation({
    mutationFn: ({
      target,
      path,
      replace,
    }: {
      target: TriggerFile
      path: string | null
      replace: boolean
    }) => installSkillTrigger(target.client, path, replace),
    refresh: [keys.skillTriggers],
    onSuccess: (written) => {
      setAsking(false)
      say.ok(t('agents.trigger.installed', { path: written }))
    },
    failure: 'toast.triggerFailed',
  })

  if (file === undefined) return null

  const onInstall = async () => {
    if (file.path === null) {
      // A client that reads it from a project: the person picks the folder.
      const destination = await save({
        defaultPath: file.file_name,
        title: t('agents.trigger.saveTitle'),
      })
      if (destination === null) return
      install.mutate({ target: file, path: destination, replace: true })
      return
    }
    if (file.exists) {
      setAsking(true)
      return
    }
    install.mutate({ target: file, path: null, replace: false })
  }

  return (
    <div className="flex flex-col gap-2">
      <SegmentedControl
        aria-label={t('agents.trigger.client')}
        value={client}
        onValueChange={(next) => setClient(next as Client)}
      >
        {files.map((one) => (
          <Segment key={one.client} value={one.client}>
            {t(`agents.trigger.clients.${one.client}`)}
          </Segment>
        ))}
      </SegmentedControl>
      <pre
        aria-label={t('agents.trigger.text')}
        className="selectable max-h-72 overflow-y-auto rounded-md border border-line bg-soft px-2.5 py-1.5 font-mono text-xs whitespace-pre-wrap"
      >
        {file.text}
      </pre>
      <p className="text-xs text-dim">
        {file.path === null
          ? t('agents.trigger.noPlace', { name: file.file_name })
          : t(file.exists ? 'agents.trigger.placeTaken' : 'agents.trigger.place', {
              path: file.path,
            })}
      </p>
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          variant="primary"
          disabled={install.isPending}
          onClick={() => void onInstall()}
        >
          {file.path === null ? t('agents.trigger.saveAs') : t('agents.trigger.install')}
        </Button>
        <CopyButton
          value={file.text}
          label={t('agents.trigger.copy')}
          copiedLabel={t('data.mcpCopied')}
          onCopy={(ok) => {
            if (!ok) say.failed(t('work.copyFailed'))
          }}
          title={t('agents.trigger.copy')}
        />
      </div>
      <ConfirmAction
        open={asking}
        onOpenChange={setAsking}
        title={t('agents.trigger.replaceTitle')}
        description={t('agents.trigger.replaceDescription', { path: file.path ?? '' })}
        actionLabel={t('agents.trigger.replace')}
        pending={install.isPending}
        onConfirm={() => install.mutate({ target: file, path: null, replace: true })}
      />
    </div>
  )
}
