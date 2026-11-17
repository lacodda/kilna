import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { compactWorkspace, examineWorkspace, restoreBackup, setHistoryDays } from '@/lib/api/upkeep'
import type { Examination, Found, KeptBackup } from '@/lib/api/types'
import { sayReason } from '@/lib/errors'
import { formatBytes, formatStamp } from '@/lib/format'
import { nameOf } from '@/lib/media'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Select } from '@/components/AppSelect'
import { ConfirmAction } from '@/components/ConfirmAction'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'

/*
 * Looking after the open space (v0.114): the copies kilna keeps of it and
 * putting one back, how much room it takes, an examination of it, and how
 * long its history is kept. Four fields of Settings → Data, each its own
 * component so the section stays a list of what is there.
 */

/** A backup to put back: one kilna kept, or a file the person picked. */
type Chosen = { path: string; kept: KeptBackup | null }

/**
 * The copies kilna keeps by itself, and putting a backup back.
 *
 * Until v0.114 a restore was four steps taken by hand with kilna closed.
 * It is a button now: the database is restored through SQLite into the open
 * file, what stood before is kept first, and the window reads everything
 * again. The steps stay under it, for a space that will not open.
 */
export function KeptBackups() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const kept = useQuery(queries.keptBackups())
  const [chosen, setChosen] = useState<Chosen | null>(null)

  const restore = useAppMutation({
    mutationFn: (path: string) => restoreBackup(path),
    failure: 'data.restoreFailed',
    onSuccess: (aside) => {
      setChosen(null)
      // Every row the window holds is of the space that is gone.
      void client.invalidateQueries()
      say.ok(t('data.restored', { path: aside }))
    },
  })

  const pick = async () => {
    try {
      const path = await open({
        title: t('data.restoreFileTitle'),
        filters: [{ name: 'kilna', extensions: ['db'] }],
      })
      if (typeof path === 'string') setChosen({ path, kept: null })
    } catch (cause) {
      say.failed(cause)
    }
  }

  const copies = kept.data ?? []

  return (
    <FieldGroup label={t('data.backups')} help={t('data.backupsHint')}>
      {kept.data !== undefined && copies.length === 0 && (
        <p className="text-sm text-dim">{t('data.backupsNone')}</p>
      )}
      {copies.length > 0 && (
        <ul className="flex flex-col rounded-md border border-line">
          {copies.map((copy) => (
            <li
              key={copy.path}
              className="flex items-center gap-2 border-b border-line px-2.5 py-1.5 last:border-b-0"
            >
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm text-text">{formatStamp(copy.madeAt)}</span>
                <span className="block truncate text-xs text-dim">
                  {t(BACKUP_KIND_KEYS[copy.kind])} · {formatBytes(copy.bytes)}
                </span>
              </span>
              <Button size="sm" onClick={() => setChosen({ path: copy.path, kept: copy })}>
                {t('data.restoreThis')}
              </Button>
            </li>
          ))}
        </ul>
      )}
      <Button className="self-start" onClick={() => void pick()}>
        {t('data.restoreFile')}
      </Button>

      <ConfirmAction
        open={chosen !== null}
        onOpenChange={(open) => {
          if (!open) setChosen(null)
        }}
        title={t('data.restoreTitle')}
        description={
          chosen === null
            ? ''
            : chosen.kept === null
              ? t('data.restoreFileBody', { name: nameOf(chosen.path) })
              : t('data.restoreBody', { when: formatStamp(chosen.kept.madeAt) })
        }
        actionLabel={t('data.restoreConfirm')}
        pending={restore.isPending}
        onConfirm={() => {
          if (chosen !== null) restore.mutate(chosen.path)
        }}
      />
    </FieldGroup>
  )
}

/** A word per kind of copy, written out so a new kind is a type error here. */
const BACKUP_KIND_KEYS: Record<KeptBackup['kind'], string> = {
  daily: 'data.backupKind.daily',
  beforeUpgrade: 'data.backupKind.beforeUpgrade',
  beforeRestore: 'data.backupKind.beforeRestore',
}

/** How much room the space takes, and giving back what the database holds
 * and no longer uses. */
export function StorageField() {
  const { t } = useTranslation()
  const storage = useQuery(queries.storage())

  const compact = useAppMutation({
    mutationFn: compactWorkspace,
    refresh: [keys.storage],
    failure: 'data.compactFailed',
    onSuccess: (done) =>
      say.ok(
        t('data.compacted', {
          before: formatBytes(done.beforeBytes),
          after: formatBytes(done.afterBytes),
        }),
      ),
  })

  const size = storage.data
  const rows: [string, number][] =
    size === undefined
      ? []
      : [
          [t('data.sizeDatabase'), size.databaseBytes],
          [t('data.sizeMedia', { count: size.mediaFiles }), size.mediaBytes],
          [t('data.sizeBackups', { count: size.backups }), size.backupsBytes],
        ]

  return (
    <FieldGroup label={t('data.size')} help={t('data.sizeHint')}>
      {rows.length > 0 && (
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
          {rows.map(([label, bytes]) => (
            <div key={label} className="contents">
              <dt className="text-dim">{label}</dt>
              <dd className="font-mono text-text">{formatBytes(bytes)}</dd>
            </div>
          ))}
        </dl>
      )}
      {size !== undefined && (
        <p className="text-sm text-dim">
          {size.reclaimableBytes > 0
            ? t('data.compactHint', { size: formatBytes(size.reclaimableBytes) })
            : t('data.compactNothing')}
        </p>
      )}
      <Button
        className="self-start"
        disabled={compact.isPending || size === undefined || size.reclaimableBytes === 0}
        onClick={() => compact.mutate()}
      >
        {t('data.compact')}
      </Button>
    </FieldGroup>
  )
}

/**
 * An examination of the space: what the database says of itself, what the
 * disk says of the files, and whether the plugins and the assistant's CLI
 * can be reached. It only looks; what to do about a lost file is said, not
 * done.
 */
export function ExamineField() {
  const { t } = useTranslation()
  const [found, setFound] = useState<Examination | null>(null)
  // Asked when the examination is: both are already answered for their own
  // screens, and a second way of asking would be a second answer.
  const plugins = useQuery({ ...queries.plugins(), enabled: found !== null })
  const cli = useQuery({ ...queries.assistantStatus(), enabled: found !== null })

  const examine = useAppMutation({
    mutationFn: examineWorkspace,
    failure: 'data.examineFailed',
    onSuccess: setFound,
  })

  const unusable = (plugins.data ?? []).filter((plugin) => !plugin.usable)
  const clean =
    found !== null &&
    found.damage.length === 0 &&
    found.dangling === 0 &&
    found.lost.count === 0 &&
    found.ownerless.count === 0 &&
    found.stray.count === 0 &&
    unusable.length === 0 &&
    cli.data?.available !== false

  return (
    <FieldGroup label={t('data.examine')} help={t('data.examineHint')}>
      <Button className="self-start" disabled={examine.isPending} onClick={() => examine.mutate()}>
        {t('data.examineAction')}
      </Button>

      {found !== null && clean && <p className="text-sm text-good">{t('data.examineClean')}</p>}
      {found !== null && !clean && (
        <ul className="flex flex-col gap-2 text-sm text-text">
          {found.damage.length > 0 && (
            <Finding title={t('data.examined.damage')} names={found.damage} />
          )}
          {found.dangling > 0 && (
            <Finding title={t('data.examined.dangling', { count: found.dangling })} names={[]} />
          )}
          <Files title="data.examined.lost" found={found.lost} />
          <Files title="data.examined.ownerless" found={found.ownerless} />
          {found.stray.count > 0 && (
            <Finding
              title={t('data.examined.stray', {
                count: found.stray.count,
                size: formatBytes(found.strayBytes),
              })}
              names={found.stray.names}
              more={found.stray.count - found.stray.names.length}
            />
          )}
          {unusable.length > 0 && (
            <Finding
              title={t('data.examined.plugins', { count: unusable.length })}
              names={unusable.map((plugin) =>
                plugin.reason === null
                  ? plugin.executable
                  : `${plugin.executable}: ${sayReason(plugin.reason)}`,
              )}
            />
          )}
          {cli.data?.available === false && (
            <Finding
              title={t('data.examined.cli')}
              names={cli.data.reason === null ? [] : [cli.data.reason]}
            />
          )}
        </ul>
      )}
    </FieldGroup>
  )
}

/** Files of one kind the examination found, when it found any. */
function Files({ title, found }: { title: string; found: Found }) {
  const { t } = useTranslation()
  if (found.count === 0) return null
  return (
    <Finding
      title={t(title, { count: found.count })}
      names={found.names}
      more={found.count - found.names.length}
    />
  )
}

/** One thing found: what it is, and the names it has. */
function Finding({ title, names, more = 0 }: { title: string; names: string[]; more?: number }) {
  const { t } = useTranslation()
  return (
    <li>
      <p className="text-warn">{title}</p>
      {names.length > 0 && (
        <ul className="mt-0.5 flex flex-col gap-0.5 pl-4 font-mono text-xs break-all text-dim">
          {names.map((name) => (
            <li key={name}>{name}</li>
          ))}
          {more > 0 && <li>{t('data.examined.more', { count: more })}</li>}
        </ul>
      )}
    </li>
  )
}

/** The lengths the history can be kept for; `0` is for good. */
const HISTORY_DAYS = [7, 30, 90, 365, 0] as const

/** How long the read lines of the history are kept, and where they go. */
export function HistoryField() {
  const { t } = useTranslation()
  const keeping = useQuery(queries.historyKeeping())

  const set = useAppMutation({
    mutationFn: (days: number) => setHistoryDays(days),
    refresh: [keys.historyKeeping],
    failure: 'data.historyFailed',
  })

  const days = keeping.data?.days
  // A length written into the space's file by hand is still a length.
  const lengths: number[] =
    days === undefined || HISTORY_DAYS.some((known) => known === days)
      ? [...HISTORY_DAYS]
      : [...HISTORY_DAYS, days]

  return (
    <FieldGroup
      label={t('data.history')}
      help={t('data.historyHint', { path: keeping.data?.archive ?? '' })}
    >
      <Select
        aria-label={t('data.history')}
        className="self-start"
        disabled={days === undefined || set.isPending}
        value={String(days ?? 7)}
        onChange={(value) => set.mutate(Number(value))}
        options={lengths.map((length) => ({
          value: String(length),
          label: length === 0 ? t('data.historyForGood') : t('data.historyDays', { count: length }),
        }))}
      />
    </FieldGroup>
  )
}
