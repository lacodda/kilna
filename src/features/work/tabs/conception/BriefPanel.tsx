import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { createVersion } from '@/lib/api/versions'
import type { Work } from '@/lib/api/types'
import { briefRoleOf } from '@/lib/conception'
import { formatDay } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useBodyEditing } from '@/lib/useBodyEditing'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Panel } from '@/components/ui/panel'
import { Textarea } from '@/components/ui/textarea'

interface Props {
  work: Work
}

/**
 * The brief, in the author's words (ADR 0065): what the work is about, where
 * it comes from, how they see it, what its text should be like - the
 * sections the profile's skeleton names, started from with one press.
 *
 * Written the way every text of a work is: the first change to the open
 * revision mints the next one, and the changes after it go into that one
 * (`useBodyEditing`). A brief is never the work's current version - it is
 * written about the work, not as it - so nothing typed here moves the text
 * the overview shows. "Refine" proposes its next revision, written from this
 * one.
 */
export function BriefPanel({ work }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const role = briefRoleOf(config, work.kind)
  const versions = useQuery(queries.versions(work.id))
  // The brief being written is its newest revision: the list comes newest
  // first within a role.
  const newest = (versions.data ?? []).find((version) => version.role === role?.key) ?? null
  const [minted, setMinted] = useState<string | null>(null)
  const openId = minted ?? newest?.id ?? null
  const open = useQuery({
    ...queries.version(openId ?? ''),
    enabled: openId !== null,
    placeholderData: keepPreviousData,
  })

  const editing = useBodyEditing({
    workId: work.id,
    role: role?.key ?? '',
    open: open.data?.id === openId ? open.data : undefined,
    onMinted: setMinted,
    failure: t('toast.versionSaveFailed'),
    current: false,
  })

  const skeleton = role?.skeleton == null ? '' : sayLabel(role.skeleton)
  const begin = useAppMutation({
    mutationFn: () =>
      createVersion(work.id, { role: role!.key, body: skeleton, make_current: false }),
    failure: 'toast.versionSaveFailed',
    refresh: refresh.version(work.id),
    onSuccess: (version) => setMinted(version.id),
  })

  if (role === undefined) return null
  const label = sayLabel(role.label)
  const shown = open.data?.id === openId ? open.data : undefined

  return (
    <Panel className="flex min-h-72 flex-col gap-1.5 px-3 py-2.5">
      <div className="flex items-center gap-2">
        <h3 className="caption">{label}</h3>
        {shown !== undefined && (
          <span className="text-2xs text-faint">
            {t('conception.revision', {
              revision: shown.revision,
              day: formatDay(shown.created_at),
            })}
          </span>
        )}
        {editing.status !== 'idle' && (
          <span className="ml-auto text-2xs text-faint" role="status">
            {t(`conception.${editing.status}`)}
          </span>
        )}
      </div>
      {openId === null ? (
        <div className="flex flex-1 flex-col items-start gap-2 rounded-lg border border-dashed border-line-2 bg-softer px-3 py-3">
          <p className="text-sm text-dim">{t('conception.briefEmpty')}</p>
          {skeleton !== '' && (
            <pre className="w-full font-sans text-xs whitespace-pre-wrap text-faint">
              {skeleton}
            </pre>
          )}
          <Button
            size="sm"
            variant="primary"
            disabled={begin.isPending || versions.isLoading}
            onClick={() => begin.mutate(undefined)}
          >
            {t('conception.begin', { brief: label.toLowerCase() })}
          </Button>
        </div>
      ) : (
        <Textarea
          aria-label={label}
          autoResize
          rows={10}
          maxRows={40}
          value={editing.text}
          onChange={(event) => editing.setText(event.target.value)}
          className="flex-1 leading-relaxed"
        />
      )}
    </Panel>
  )
}
