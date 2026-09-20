import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import {
  listStyleBricks,
  styleBrickCounts,
  type StyleBrick,
  type StyleType,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { useProfile, styleTypesOf, say } from '@/lib/useProfile'
import { styleIconOf } from '@/lib/styleIcon'
import { useDebounced } from '@/lib/useDebounced'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/ui/EmptyState'
import { SkeletonList } from '@/components/ui/Skeleton'
import { StyleBrickDialog } from '@/components/styles/StyleBrickDialog'
import { StyleBrickCard } from '@/components/styles/StyleBrickCard'
import { cn } from '@/lib/utils'

/**
 * The workspace's style dictionary: the parts a picture prompt is built from.
 *
 * One screen for all of them, grouped by the type the profile names, because
 * the whole point of ADR 0031 is that there is one dictionary and not several.
 * A craft that names no types has none, and the rail does not offer this.
 */
export function StylesView() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const { config } = useProfile()
  const types = styleTypesOf(config)

  const [typeKey, setTypeKey] = useState<string | undefined>(undefined)
  const [text, setText] = useState('')
  // The list is a query; typing into it unthrottled would refetch per letter.
  const query = useDebounced(text, 200)
  const [editing, setEditing] = useState<StyleBrick | 'new' | null>(null)

  const bricks = useQuery({
    queryKey: [...keys.styleBricks, typeKey ?? null, query],
    queryFn: () => listStyleBricks({ type_key: typeKey ?? null, query: query || null }),
  })
  const counts = useQuery({ queryKey: keys.styleCounts, queryFn: styleBrickCounts })

  const countOf = useMemo(
    () => new Map(counts.data ?? []),
    [counts.data],
  )
  const total = useMemo(
    () => (counts.data ?? []).reduce((sum, [, n]) => sum + n, 0),
    [counts.data],
  )

  const settle = () => {
    void client.invalidateQueries({ queryKey: keys.styles })
  }

  // Grouped in the profile's order of types, which the backend already sorted
  // the rows into: the document's order is the reading order.
  const groups = useMemo(() => {
    const rows = bricks.data ?? []
    const out: { type: StyleType | undefined; key: string; rows: StyleBrick[] }[] = []
    for (const row of rows) {
      const last = out[out.length - 1]
      if (last !== undefined && last.key === row.type_key) {
        last.rows.push(row)
        continue
      }
      out.push({
        key: row.type_key,
        type: types.find((one) => one.key === row.type_key),
        rows: [row],
      })
    }
    return out
  }, [bricks.data, types])

  if (types.length === 0) {
    return (
      <EmptyState
        title={t('styles.noDictionary')}
        body={t('styles.noDictionaryBody')}
      />
    )
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Input
          value={text}
          onChange={(event) => setText(event.target.value)}
          placeholder={t('styles.search')}
          aria-label={t('styles.search')}
          className="max-w-xs"
        />
        <Button variant="primary" onClick={() => setEditing('new')} className="ml-auto">
          <Plus aria-hidden />
          {t('styles.new')}
        </Button>
      </div>

      {/* The types as a row of chips, the way the storyboard narrows to a kind
          of shot: "show me every environment" is one click, and the chip that
          is on turns off. */}
      <div role="group" aria-label={t('styles.type')} className="flex flex-wrap items-center gap-2">
        {[
          { key: undefined, label: t('styles.allTypes'), count: total, type: undefined },
          ...types.map((one) => ({
            key: one.key,
            label: say(one.label),
            count: countOf.get(one.key) ?? 0,
            type: one,
          })),
        ].map((entry) => {
          const active = typeKey === entry.key
          const Icon = entry.type === undefined ? undefined : styleIconOf(entry.type)
          return (
            <button
              key={entry.key ?? ''}
              type="button"
              aria-pressed={active}
              onClick={() => setTypeKey(active ? undefined : entry.key)}
              className={cn(
                'flex cursor-pointer items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11.5px] transition-colors',
                active
                  ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                  : 'border-line text-dim hover:border-line-2 hover:text-text',
              )}
            >
              {Icon !== undefined && <Icon aria-hidden className="size-3.5" />}
              {entry.label}
              <span className="text-[10.5px] text-faint tabular-nums">{entry.count}</span>
            </button>
          )
        })}
      </div>

      {bricks.isPending ? (
        <SkeletonList rows={4} />
      ) : groups.length === 0 ? (
        <EmptyState
          title={query === '' ? t('styles.empty') : t('styles.noMatches')}
          body={query === '' ? t('styles.emptyBody') : undefined}
          action={
            query === '' ? (
              <Button variant="primary" onClick={() => setEditing('new')}>
                <Plus aria-hidden />
                {t('styles.new')}
              </Button>
            ) : undefined
          }
        />
      ) : (
        groups.map((group) => (
          <section key={group.key} className="flex flex-col gap-2">
            <h2 className="flex items-center gap-1.5 text-2xs font-semibold uppercase tracking-caption text-faint">
              {(() => {
                const Icon = styleIconOf(group.type)
                return <Icon aria-hidden className="size-3.5" />
              })()}
              {group.type === undefined ? group.key : say(group.type.label)}
            </h2>
            <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 xl:grid-cols-3">
              {group.rows.map((brick) => (
                <StyleBrickCard
                  key={brick.id}
                  brick={brick}
                  onOpen={() => setEditing(brick)}
                />
              ))}
            </div>
          </section>
        ))
      )}

      {editing !== null && (
        <StyleBrickDialog
          open
          brick={editing === 'new' ? undefined : editing}
          types={types}
          onOpenChange={(open) => {
            if (!open) setEditing(null)
          }}
          onSettled={settle}
        />
      )}
    </div>
  )
}
