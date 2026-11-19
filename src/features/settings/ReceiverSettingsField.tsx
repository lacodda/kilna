import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import type { ReceiverSettings } from '@/lib/api/types'
import { setReceiverSettings } from '@/lib/api/receiver'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Field, FieldGroup } from '@/components/ui/field'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Switch } from '@/components/ui/switch'

const WAITS = ['5', '15', '30', '60']

/**
 * How the armed receiver works on this machine (v0.156, FR-21 21.2): the
 * folder it waits in - the system's Downloads unless another is chosen -,
 * how long it waits, and whether what arrives is moved in without asking.
 *
 * Here, beside the theme, rather than in the space: a Downloads folder is a
 * machine's, and the same on every space opened on it.
 */
export function ReceiverSettingsField() {
  const { t } = useTranslation()
  const kept = useQuery(queries.receiverSettings())

  const keep = useAppMutation({
    mutationFn: (next: ReceiverSettings) => setReceiverSettings(next),
    failure: 'receiver.settingsFailed',
    refresh: [keys.receiverSettings],
  })

  const settings = kept.data
  if (settings === undefined) return null

  const choose = async () => {
    try {
      const chosen = await open({
        directory: true,
        title: t('receiver.folderTitle'),
        defaultPath: settings.folder ?? undefined,
      })
      if (typeof chosen === 'string') keep.mutate({ ...settings, folder: chosen })
    } catch (cause) {
      say.failed(cause)
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <h3 className="caption">{t('receiver.title')}</h3>
      <FieldGroup label={t('receiver.folder')} help={t('receiver.folderHint')}>
        <div className="flex items-center gap-2">
          <span className="min-w-0 flex-1 truncate text-sm">
            {settings.folder ?? t('receiver.systemDownloads')}
          </span>
          <Button size="sm" onClick={() => void choose()}>
            {t('receiver.chooseFolder')}
          </Button>
          {settings.folder !== null && (
            <Button
              size="sm"
              variant="ghost"
              onClick={() => keep.mutate({ ...settings, folder: null })}
            >
              {t('receiver.useDownloads')}
            </Button>
          )}
        </div>
      </FieldGroup>
      <FieldGroup label={t('receiver.minutes')}>
        <SegmentedControl
          aria-label={t('receiver.minutes')}
          value={String(settings.minutes)}
          onValueChange={(next) => keep.mutate({ ...settings, minutes: Number(next) })}
        >
          {(WAITS.includes(String(settings.minutes))
            ? WAITS
            : [...WAITS, String(settings.minutes)]
          ).map((minutes) => (
            <Segment key={minutes} value={minutes}>
              {t('receiver.minutesValue', { count: Number(minutes) })}
            </Segment>
          ))}
        </SegmentedControl>
      </FieldGroup>
      <Field label={t('receiver.auto')} help={t('receiver.autoHint')} beside>
        <Switch
          checked={settings.auto}
          disabled={keep.isPending}
          onCheckedChange={(auto) => keep.mutate({ ...settings, auto })}
        />
      </Field>
    </div>
  )
}
