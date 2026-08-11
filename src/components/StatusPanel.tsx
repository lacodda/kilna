import { useTranslation } from 'react-i18next'
import type { Workspace } from '@/lib/api'

interface Props {
  workspace: Workspace
}

export function StatusPanel({ workspace }: Props) {
  const { t } = useTranslation()
  const { profile } = workspace

  return (
    <section className="flex flex-col gap-6">
      <dl className="grid grid-cols-2 gap-x-8 gap-y-3 text-sm sm:grid-cols-3">
        <Field label={t('status.profile')} value={profile?.name ?? '—'} />
        <Field label={t('status.schema')} value={String(workspace.schema_version)} />
        <Field label={t('status.works')} value={String(workspace.works)} />
      </dl>

      {profile !== null && (
        <div className="flex flex-wrap gap-2">
          {profile.config.axes.map((axis) => (
            <span
              key={axis.key}
              title={axis.description}
              className="rounded bg-neutral-100 px-2 py-1 text-xs text-neutral-700 dark:bg-neutral-800 dark:text-neutral-200"
            >
              {axis.label} ×{axis.weight}
            </span>
          ))}
        </div>
      )}

      <div className="rounded-lg border border-dashed border-neutral-300 p-8 text-center dark:border-neutral-700">
        <p className="font-medium">{t('empty.title')}</p>
        <p className="mt-1 text-sm text-neutral-500">{t('empty.body')}</p>
      </div>
    </section>
  )
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wide text-neutral-500">{label}</dt>
      <dd className="mt-0.5 font-medium">{value}</dd>
    </div>
  )
}
