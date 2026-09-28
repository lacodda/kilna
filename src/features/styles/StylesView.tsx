import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import type { StyleBrick, StyleType } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useProfile, styleTypesOf, say } from '@/lib/useProfile'
import { styleIconOf } from '@/lib/styleIcon'
import { useDebounced } from '@/lib/useDebounced'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { StyleBrickDialog } from '@/features/styles/StyleBrickDialog'
import { StyleBrickCard } from '@/features/styles/StyleBrickCard'

/*
 * What a type chip is called in its group. "All" is a chip of its own, and the
 * types carry a prefix, so a profile that names a type "all" cannot be taken
 * for it.
 */
const ALL_TYPES = 'all'
const chipOf = (type: string | undefined) => (type === undefined ? ALL_TYPES : `type:${type}`)
const typeOf = (chip: string | undefined) =>
  chip === undefined || chip === ALL_TYPES ? undefined : chip.slice('type:'.length)

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

  const bricks = useQuery(queries.styleBricksMatching(typeKey ?? null, query))
  const counts = useQuery(queries.styleCounts())

  const countOf = useMemo(() => new Map(counts.data ?? []), [counts.data])
  const total = useMemo(() => (counts.data ?? []).reduce((sum, [, n]) => sum + n, 0), [counts.data])

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
      <Frame>
        <EmptyState
          title={t('styles.noDictionary')}
          body={t('styles.noDictionaryBody')}
          className="flex-1"
        />
      </Frame>
    )
  }

  return (
    <Frame
      head={
        <>
          {/* The types as a row of chips, the way the storyboard narrows to a kind
              of shot: "show me every environment" is one click, and the chip that
              is on turns off - letting go of one leaves the group empty, which is
              every type. */}
          <ChipGroup
            aria-label={t('styles.type')}
            value={[chipOf(typeKey)]}
            onValueChange={(next) => setTypeKey(typeOf(next[0]))}
          >
            <Chip value={chipOf(undefined)} count={total}>
              {t('styles.allTypes')}
            </Chip>
            {types.map((one) => {
              const Icon = styleIconOf(one)
              return (
                <Chip key={one.key} value={chipOf(one.key)} count={countOf.get(one.key) ?? 0}>
                  <Icon aria-hidden className="size-3.5" />
                  {say(one.label)}
                </Chip>
              )
            })}
          </ChipGroup>
          <Input
            value={text}
            onChange={(event) => setText(event.target.value)}
            placeholder={t('styles.search')}
            aria-label={t('styles.search')}
            className="max-w-60"
          />
          <Button variant="primary" onClick={() => setEditing('new')} className="ml-auto">
            <Plus aria-hidden />
            {t('styles.new')}
          </Button>
        </>
      }
    >
      <Loaded
        query={bricks}
        fill
        skeleton={<SkeletonList rows={4} />}
        isEmpty={() => groups.length === 0}
        emptyState={
          query === '' ? (
            <EmptyState
              title={t('styles.empty')}
              body={t('styles.emptyBody')}
              action={
                <Button variant="primary" onClick={() => setEditing('new')}>
                  <Plus aria-hidden />
                  {t('styles.new')}
                </Button>
              }
              className="flex-1"
            />
          ) : (
            // A search that matched nothing: the way out is the box above.
            <EmptyState variant="filtered" title={t('styles.noMatches')} className="flex-1" />
          )
        }
      >
        {() => (
          <Scroll label={t('nav.styles')} contentClassName="flex flex-col gap-4">
            {groups.map((group) => (
              <section key={group.key} className="flex flex-col gap-2">
                <h2 className="flex items-center gap-1.5 caption">
                  {(() => {
                    const Icon = styleIconOf(group.type)
                    return <Icon aria-hidden className="size-3.5" />
                  })()}
                  {group.type === undefined ? group.key : say(group.type.label)}
                </h2>
                <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 xl:grid-cols-3">
                  {group.rows.map((brick) => (
                    <StyleBrickCard key={brick.id} brick={brick} onOpen={() => setEditing(brick)} />
                  ))}
                </div>
              </section>
            ))}
          </Scroll>
        )}
      </Loaded>

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
    </Frame>
  )
}
