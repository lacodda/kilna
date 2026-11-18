import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import type { BrickStanding, ListedPack, PackStanding, Source } from '@/lib/api/types'
import { installStylePack, stylePack } from '@/lib/api/packs'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { say as sayLabel } from '@/lib/useProfile'
import { Dialog } from '@/components/AppDialog'
import { Select } from '@/components/AppSelect'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'

const HALVES = ['picture', 'motion', 'sound'] as const

const PACK_BADGE: Record<PackStanding, 'soft' | 'good' | 'accent' | 'outline'> = {
  absent: 'outline',
  installed: 'good',
  update: 'accent',
  unfit: 'soft',
}

/**
 * Packs of styles from the registries this machine reads (v0.131, FR-06):
 * the snapshot shipped with kilna, and the folders and sites added in
 * Settings. Narrowed by half, category and words; a pack opens into its
 * bricks, each with where it stands in the open space, and is installed
 * whole or one brick at a time. A brick its owner changed keeps their words;
 * one they deleted comes back only when it is installed by hand.
 */
export function PacksDialog({
  open,
  onOpenChange,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const { t } = useTranslation()
  const catalogue = useQuery({ ...queries.stylePacks(), enabled: open })
  const [half, setHalf] = useState<string>('all')
  const [category, setCategory] = useState('')
  const [text, setText] = useState('')
  const [opened, setOpened] = useState<{ source: Source; pack: string } | null>(null)

  const packs = catalogue.data?.packs ?? []
  const categories = [...new Set(packs.map((pack) => pack.entry.category))]
  const needle = text.trim().toLowerCase()
  const shown = packs.filter(
    (pack) =>
      (half === 'all' || pack.entry.half === half) &&
      (category === '' || pack.entry.category === category) &&
      (needle === '' ||
        [sayLabel(pack.entry.title), sayLabel(pack.entry.description), pack.entry.id].some((word) =>
          word.toLowerCase().includes(needle),
        )),
  )

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) setOpened(null)
        onOpenChange(next)
      }}
      title={t('packs.title')}
      description={t('packs.description')}
      size="lg"
    >
      {opened !== null ? (
        <Opened source={opened.source} pack={opened.pack} onBack={() => setOpened(null)} />
      ) : (
        <div className="flex flex-col gap-3">
          <div className="flex flex-wrap items-center gap-2">
            <SegmentedControl aria-label={t('packs.half')} value={half} onValueChange={setHalf}>
              <Segment value="all">{t('packs.halves.all')}</Segment>
              {HALVES.map((one) => (
                <Segment key={one} value={one}>
                  {t(`packs.halves.${one}`)}
                </Segment>
              ))}
            </SegmentedControl>
            <Select
              className="w-48"
              aria-label={t('packs.category')}
              placeholder={t('packs.anyCategory')}
              value={category}
              onChange={setCategory}
              options={categories.map((one) => ({
                value: one,
                label: t(`packs.categories.${one}`, { defaultValue: one }),
              }))}
            />
            <Input
              className="w-56"
              type="search"
              value={text}
              placeholder={t('packs.search')}
              aria-label={t('packs.search')}
              onChange={(event) => setText(event.target.value)}
            />
          </div>
          {(catalogue.data?.failed ?? []).map((failure, index) => (
            <p key={index} className="text-xs text-warn">
              {failure}
            </p>
          ))}
          {catalogue.data?.offline === true && (
            <p className="text-xs text-dim">{t('packs.offlineNote')}</p>
          )}
          <ul className="flex flex-col gap-2">
            {shown.map((pack) => (
              <PackRow
                key={`${pack.source.kind}:${pack.entry.id}`}
                pack={pack}
                onOpen={() => setOpened({ source: pack.source, pack: pack.entry.id })}
              />
            ))}
          </ul>
          {catalogue.data !== undefined && shown.length === 0 && (
            <p className="text-sm text-dim">{t('packs.none')}</p>
          )}
        </div>
      )}
    </Dialog>
  )
}

function PackRow({ pack, onOpen }: { pack: ListedPack; onOpen: () => void }) {
  const { t } = useTranslation()
  return (
    <li className="flex items-start gap-3 rounded-md border border-line p-3">
      <div className="min-w-0 flex-1">
        <h3 className="flex flex-wrap items-baseline gap-x-2 text-sm font-medium">
          {sayLabel(pack.entry.title)}
          <span className="text-xs font-normal text-faint">
            {t(`packs.halves.${pack.entry.half}`, { defaultValue: pack.entry.half })} ·{' '}
            {t('packs.bricks', { count: pack.entry.brick_count })}
          </span>
        </h3>
        <p className="text-xs text-dim">{sayLabel(pack.entry.description)}</p>
      </div>
      <Badge variant={PACK_BADGE[pack.standing]}>{t(`packs.standing.${pack.standing}`)}</Badge>
      <Button size="sm" onClick={onOpen}>
        {t('packs.open')}
      </Button>
    </li>
  )
}

const BRICK_BADGE: Record<BrickStanding, 'soft' | 'good' | 'accent' | 'outline' | 'warn'> = {
  absent: 'outline',
  installed: 'good',
  update: 'accent',
  changed: 'soft',
  deleted: 'warn',
  unfit: 'soft',
}

function Opened({ source, pack, onBack }: { source: Source; pack: string; onBack: () => void }) {
  const { t } = useTranslation()
  const opened = useQuery({
    queryKey: [...keys.stylePacks, 'opened', source.kind, pack],
    queryFn: () => stylePack(source, pack),
  })
  const install = useAppMutation({
    mutationFn: (only: string[] | null) => installStylePack(source, pack, only),
    refresh: [keys.styles, keys.workspace],
    onSuccess: (done) =>
      say.ok(t('packs.installed', { added: done.added, renewed: done.renewed, kept: done.kept })),
    failure: 'packs.installFailed',
  })
  if (opened.data === undefined) return null
  const { entry, bricks } = opened.data
  const fits = bricks.some(([, standing]) => standing !== 'unfit')

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="ghost" onClick={onBack}>
          {t('packs.back')}
        </Button>
        <h3 className="flex-1 text-sm font-medium">{sayLabel(entry.title)}</h3>
        <Button
          size="sm"
          variant="primary"
          disabled={!fits || install.isPending}
          onClick={() => install.mutate(null)}
        >
          {t('packs.installPack')}
        </Button>
      </div>
      <p className="text-xs text-dim">{sayLabel(entry.description)}</p>
      {!fits && <p className="text-xs text-warn">{t('packs.unfitNote')}</p>}
      <ul className="flex flex-col gap-2">
        {bricks.map(([brick, standing]) => (
          <li
            key={brick.key}
            className="flex items-start gap-3 rounded-md border border-line p-2.5"
          >
            <div className="min-w-0 flex-1">
              <p className="text-sm">{sayLabel(brick.label)}</p>
              <p className="font-mono text-xs text-dim">{brick.description}</p>
              {brick.when != null && <p className="text-xs text-faint">{brick.when}</p>}
            </div>
            <Badge variant={BRICK_BADGE[standing]}>{t(`packs.brickStanding.${standing}`)}</Badge>
            <Button
              size="sm"
              disabled={
                install.isPending ||
                standing === 'installed' ||
                standing === 'unfit' ||
                standing === 'changed'
              }
              onClick={() => install.mutate([brick.key])}
            >
              {standing === 'update' ? t('packs.takeNewer') : t('packs.installOne')}
            </Button>
          </li>
        ))}
      </ul>
    </div>
  )
}
