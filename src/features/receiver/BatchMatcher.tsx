import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open as chooseFiles } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { FileAudio, FileVideo, File as FileIcon } from 'lucide-react'
import type { MediaBatchReport, MediaMatch, MediaRole } from '@/lib/api/types'
import { createChat, startRun } from '@/lib/api/assistant'
import { matchMedia, mediaPreview, takeMediaBatch } from '@/lib/api/matcher'
import { formatNumber } from '@/lib/format'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Dialog } from '@/components/AppDialog'
import { Select } from '@/components/AppSelect'
import { ConfirmAction } from '@/components/ConfirmAction'
import { Button } from '@/components/ui/button'

/** What a row does when the batch is sorted: taken in as a role, left where
 *  it is, or let go into the recycle bin. */
type Fate = MediaRole | 'skip' | 'trash'

interface Row {
  match: MediaMatch
  workId: string
  fate: Fate
}

const ROLES: MediaRole[] = [
  'final_audio',
  'candidate_audio',
  'final_cover',
  'candidate_cover',
  'video',
  'asset',
]

interface Props {
  /** The collection whose works the files are matched against; every work
   *  of the space when none. */
  collectionId: string | null
  collectionTitle?: string
  onClose: () => void
}

/**
 * A batch of files sorted into their works (v0.157, FR-21 21.2 p.2): takes
 * and covers dropped in a heap, matched without guessing - by the work's id
 * in a name, its title, the first words of its cover's prompt, a track's
 * tags - and shown in a table to read and correct. Nothing moves until
 * **Sort into folders**; a duplicate is left where it is unless the person
 * lets it go into the recycle bin. Pictures named by a moment alone go to
 * the assistant on a press, which answers with a proposal of its own.
 */
export function BatchMatcher({ collectionId, collectionTitle, onClose }: Props) {
  const { t, i18n } = useTranslation()
  const catalogue = useQuery(queries.catalogue())
  const [rows, setRows] = useState<Row[] | null>(null)
  const [report, setReport] = useState<MediaBatchReport | null>(null)
  const [confirming, setConfirming] = useState(false)

  const works = useMemo(
    () =>
      (catalogue.data ?? [])
        .filter((row) => collectionId === null || row.collection_id === collectionId)
        .map((row) => ({ value: row.work_id, label: row.title }))
        .sort((a, b) => a.label.localeCompare(b.label)),
    [catalogue.data, collectionId],
  )

  const match = useAppMutation({
    mutationFn: (paths: string[]) => matchMedia(paths, collectionId),
    failure: 'matcher.matchFailed',
    onSuccess: (found) => {
      setRows((before) => [
        ...(before ?? []),
        ...found
          .filter((one) => !(before ?? []).some((row) => row.match.path === one.path))
          .map((one) => ({
            match: one,
            workId: one.work_id ?? '',
            fate: (one.duplicate ? 'skip' : one.role) as Fate,
          })),
      ])
    },
  })

  // Files dropped on the window while the matcher is open are its batch.
  useEffect(() => {
    let stop: (() => void) | undefined
    let gone = false
    void getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === 'drop' && payload.paths.length > 0) match.mutate(payload.paths)
      })
      .then((unlisten) => {
        if (gone) unlisten()
        else stop = unlisten
      })
    return () => {
      gone = true
      stop?.()
    }
    // The mutation's identity changes each render; the drop needs none of it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const choose = async () => {
    try {
      const chosen = await chooseFiles({ multiple: true, title: t('matcher.chooseTitle') })
      if (Array.isArray(chosen) && chosen.length > 0) match.mutate(chosen)
      else if (typeof chosen === 'string') match.mutate([chosen])
    } catch (cause) {
      say.failed(cause)
    }
  }

  const sort = useAppMutation({
    mutationFn: (all: Row[]) =>
      takeMediaBatch(
        all
          .filter((row) => row.fate !== 'skip' && row.fate !== 'trash' && row.workId !== '')
          .map((row) => ({
            path: row.match.path,
            work_id: row.workId,
            role: row.fate as MediaRole,
          })),
        all.filter((row) => row.fate === 'trash').map((row) => row.match.path),
      ),
    failure: 'matcher.sortFailed',
    refresh: [...refresh.works, keys.covers, keys.folders],
    onSuccess: (done) => {
      setConfirming(false)
      setReport(done)
    },
  })

  // The pictures nothing could place: what the assistant is asked about.
  const blind = (rows ?? []).filter((row) => row.match.kind === 'image' && row.workId === '')
  const ask = useAppMutation({
    mutationFn: async () => {
      const chat = await createChat({ title: t('matcher.chatTitle') })
      const reasons = i18n.language.startsWith('ru') ? 'in Russian' : 'in English'
      const prompt = [
        'Match each of these pictures to the work it is the cover of, by what it shows.',
        collectionId === null
          ? 'Read the works of the space and what their covers are drawn from with the kilna tools (`works`, `cover`).'
          : `Read the works of the collection "${collectionTitle ?? collectionId}" (${collectionId}) and what their covers are drawn from with the kilna tools (\`works\`, \`cover\`).`,
        'Pictures:',
        ...blind.map((row) => `- ${row.match.path}`),
        `Then call \`propose_media\` once with every picture you can place: role \`final_cover\` (or \`candidate_cover\` when another picture of the batch is already that work's cover), a \`reason\` - one short sentence ${reasons} about what in the picture says so - and a \`confidence\` from 0 to 1. Leave out a picture you cannot place, and say which in your answer.`,
      ].join('\n')
      await startRun(chat.id, prompt)
      return chat
    },
    failure: 'matcher.askFailed',
    refresh: [keys.allChats],
    onSuccess: () => say.ok(t('matcher.asked', { count: blind.length })),
  })

  const set = (path: string, change: Partial<Row>) =>
    setRows((before) =>
      (before ?? []).map((row) => (row.match.path === path ? { ...row, ...change } : row)),
    )

  const placed = (rows ?? []).filter(
    (row) => row.fate !== 'skip' && row.fate !== 'trash' && row.workId !== '',
  ).length
  const letGo = (rows ?? []).filter((row) => row.fate === 'trash').length

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !sort.isPending) onClose()
      }}
      title={
        collectionTitle === undefined
          ? t('matcher.title')
          : t('matcher.titleOf', { title: collectionTitle })
      }
      size="xl"
      dismiss={report !== null ? 'close' : 'cancel'}
      footer={
        report === null && rows !== null && rows.length > 0 ? (
          <>
            {blind.length > 0 && (
              <Button disabled={ask.isPending} onClick={() => ask.mutate()}>
                {t('matcher.ask', { count: blind.length })}
              </Button>
            )}
            <Button
              variant="primary"
              disabled={placed + letGo === 0 || sort.isPending}
              onClick={() => (letGo > 0 ? setConfirming(true) : sort.mutate(rows))}
            >
              {t('matcher.sort', { count: placed })}
            </Button>
          </>
        ) : undefined
      }
    >
      {report !== null ? (
        <div className="flex flex-col gap-2 text-sm">
          <p>{t('matcher.done', { taken: report.taken, trashed: report.trashed })}</p>
          {report.failed.length > 0 && (
            <ul className="flex flex-col gap-0.5 text-xs">
              {report.failed.map((one) => (
                <li key={one.name} className="flex gap-1.5">
                  <span className="text-bad">✕</span>
                  <b className="font-semibold">{one.name}</b>
                  <span className="text-dim">{one.reason}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : rows === null || rows.length === 0 ? (
        <div className="flex flex-col items-center gap-3 rounded-md border border-dashed border-line px-6 py-10 text-center">
          <p className="text-sm text-dim">{t('matcher.empty')}</p>
          <Button disabled={match.isPending} onClick={() => void choose()}>
            {t('matcher.choose')}
          </Button>
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-dim">
                <th className="w-14 pb-1 font-medium" />
                <th className="pb-1 font-medium">{t('matcher.file')}</th>
                <th className="pb-1 font-medium">{t('matcher.work')}</th>
                <th className="pb-1 font-medium">{t('matcher.fate')}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.match.path} className="border-t border-line align-middle">
                  <td className="py-1.5 pr-2">
                    <Preview match={row.match} />
                  </td>
                  <td className="py-1.5 pr-2">
                    <div className="flex min-w-0 flex-col">
                      <span className="truncate font-medium" title={row.match.path}>
                        {row.match.name}
                      </span>
                      <span className="text-xs text-dim">
                        {[
                          row.match.by === null
                            ? row.match.ambiguous.length > 0
                              ? t('matcher.ambiguous', { count: row.match.ambiguous.length })
                              : t('matcher.unmatched')
                            : t(`matcher.by.${row.match.by}`),
                          row.match.confidence === null
                            ? null
                            : `${formatNumber(row.match.confidence * 100, 0)}%`,
                          row.match.duplicate ? t('matcher.duplicate') : null,
                        ]
                          .filter((part) => part !== null)
                          .join(' · ')}
                      </span>
                    </div>
                  </td>
                  <td className="py-1.5 pr-2">
                    <Select
                      aria-label={t('matcher.workOf', { name: row.match.name })}
                      value={row.workId}
                      onChange={(workId) => set(row.match.path, { workId })}
                      options={[{ value: '', label: t('matcher.noWork') }, ...works]}
                    />
                  </td>
                  <td className="py-1.5">
                    <Select
                      aria-label={t('matcher.fateOf', { name: row.match.name })}
                      value={row.fate}
                      onChange={(fate) => set(row.match.path, { fate: fate as Fate })}
                      options={[
                        ...ROLES.map((role) => ({
                          value: role,
                          label: t(`receiver.what.${role}`),
                        })),
                        { value: 'skip', label: t('matcher.skip') },
                        { value: 'trash', label: t('matcher.trash') },
                      ]}
                    />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <div>
            <Button size="sm" variant="ghost" onClick={() => void choose()}>
              {t('matcher.chooseMore')}
            </Button>
          </div>
        </div>
      )}
      <ConfirmAction
        open={confirming}
        onOpenChange={setConfirming}
        title={t('matcher.trashTitle', { count: letGo })}
        description={t('matcher.trashText')}
        actionLabel={t('matcher.trashAction', { count: letGo })}
        pending={sort.isPending}
        onConfirm={() => {
          if (rows !== null) sort.mutate(rows)
        }}
      />
    </Dialog>
  )
}

/** A picture's row shows the picture; a track or a video its kind. */
function Preview({ match }: { match: MediaMatch }) {
  const [url, setUrl] = useState<string | null>(null)
  useEffect(() => {
    if (match.kind !== 'image') return
    let made: string | null = null
    let gone = false
    void mediaPreview(match.path)
      .then((bytes) => {
        if (gone) return
        made = URL.createObjectURL(new Blob([bytes]))
        setUrl(made)
      })
      .catch(() => undefined)
    return () => {
      gone = true
      if (made !== null) URL.revokeObjectURL(made)
    }
  }, [match.kind, match.path])
  if (match.kind === 'image')
    return url === null ? (
      <div className="size-12 rounded bg-raise" />
    ) : (
      <img src={url} alt="" className="size-12 rounded object-cover" />
    )
  const Icon = match.kind === 'audio' ? FileAudio : match.kind === 'video' ? FileVideo : FileIcon
  return <Icon className="size-6 text-dim" aria-hidden />
}
