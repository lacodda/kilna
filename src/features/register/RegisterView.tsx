import { useMemo, useState } from 'react'
import { useNavigate, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import type { RegisterEntry, TermKind } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { FACETS, inFacet, strictnessStatus, TERM_KINDS, type Facet } from '@/lib/register'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { EmptyState } from '@/components/ui/empty-state'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { SkeletonList } from '@/components/ui/skeleton'
import { StatusDot } from '@/components/ui/status-dot'
import { Select } from '@/components/AppSelect'
import { Frame, ListDetail, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { NewTermDialog } from '@/features/register/NewTermDialog'
import { TermDetail } from '@/features/register/TermDetail'

/**
 * The register of repeats (ADR 0044): what the works have spent, strictest
 * first, each term with how many works carry it now - and, since one word is
 * one record (ADR 0052), the same list read by its other facets: the words
 * kept in the bank, the words sung their own way.
 *
 * The count is the register's reason to be read by eye, so it stands at the
 * end of every row and the list can be ordered by it. It is read off the
 * works' current texts each time the list is asked for - nothing here stores
 * it - so a verse rewritten a minute ago is already counted.
 *
 * The open term is part of the address, so a term in a text's strip opens
 * here directly and back walks between terms.
 */
export function RegisterView() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { termId } = useParams()
  const terms = useQuery(queries.terms())

  const [facet, setFacet] = useState<Facet>('all')
  const [kind, setKind] = useState<TermKind | ''>('')
  const [text, setText] = useState('')
  const [adding, setAdding] = useState(false)

  const all = useMemo(() => terms.data ?? [], [terms.data])
  const counts = useMemo(() => {
    const map = new Map<Facet, number>()
    for (const one of FACETS) map.set(one, all.filter((entry) => inFacet(entry, one)).length)
    return map
  }, [all])

  // Narrowed here: the register is one list the screen already holds, and a
  // hundred and fifty terms filter faster than a round trip.
  const rows = useMemo(() => {
    const needle = text.trim().toLowerCase()
    return all
      .filter(
        (entry) =>
          inFacet(entry, facet) &&
          (kind === '' || entry.kind === kind) &&
          (needle === '' || matches(entry, needle)),
      )
      .sort((a, b) => b.uses - a.uses || a.word.localeCompare(b.word))
  }, [all, facet, kind, text])

  const open = (id: string | null) => void navigate(id === null ? '/register' : `/register/${id}`)
  const selected = all.find((entry) => entry.id === termId)
  const filtered = facet !== 'all' || kind !== '' || text.trim() !== ''

  return (
    <Frame
      head={
        <>
          <ChipGroup
            aria-label={t('register.facet')}
            value={[facet]}
            onValueChange={(next) => setFacet((next[0] as Facet | undefined) ?? 'all')}
          >
            {FACETS.map((one) => (
              <Chip key={one} value={one} count={counts.get(one) ?? 0}>
                {t(`register.facets.${one}`)}
              </Chip>
            ))}
          </ChipGroup>
          <Select
            value={kind}
            onChange={(next) => setKind(next as TermKind | '')}
            placeholder={t('register.anyKind')}
            options={TERM_KINDS.map((one) => ({ value: one, label: t(`register.kinds.${one}`) }))}
            aria-label={t('register.kind')}
            className="w-44"
          />
          <Input
            value={text}
            onChange={(event) => setText(event.target.value)}
            placeholder={t('register.search')}
            aria-label={t('register.search')}
            className="w-56"
          />
          <Button variant="primary" className="ml-auto" onClick={() => setAdding(true)}>
            <Plus aria-hidden />
            {t('register.new')}
          </Button>
        </>
      }
    >
      <ListDetail
        list={
          <Pane label={t('nav.register')} bodyClassName="p-1.5">
            <Loaded
              query={terms}
              skeleton={<SkeletonList rows={6} />}
              isEmpty={() => rows.length === 0}
              emptyState={
                <EmptyState
                  plain
                  variant={filtered ? 'filtered' : 'empty'}
                  title={filtered ? t('register.noMatches') : t('register.empty')}
                  className="p-2"
                />
              }
              plain
            >
              {() => (
                <ul className="flex flex-col gap-0.5">
                  {rows.map((entry) => (
                    <li key={entry.id}>
                      <RowButton
                        selected={entry.id === termId}
                        onClick={() => open(entry.id)}
                        start={
                          entry.strictness === null ? (
                            <StatusDot status="neutral" label={t('register.notSpent')} />
                          ) : (
                            <StatusDot
                              status={strictnessStatus(entry.strictness)}
                              label={t(`register.strictnesses.${entry.strictness}`)}
                            />
                          )
                        }
                        description={describe(entry, t)}
                        end={
                          <span
                            className="tabular-nums"
                            title={t('register.uses', { count: entry.uses })}
                          >
                            {entry.uses}
                          </span>
                        }
                      >
                        {entry.word}
                      </RowButton>
                    </li>
                  ))}
                </ul>
              )}
            </Loaded>
          </Pane>
        }
        detail={
          selected !== undefined ? (
            <TermDetail key={selected.id} entry={selected} onGone={() => open(null)} />
          ) : termId !== undefined && !terms.isPending ? (
            <EmptyState
              title={t('register.gone')}
              body={t('register.goneBody')}
              className="flex-1"
            />
          ) : (
            <EmptyState
              className="flex-1"
              title={all.length === 0 ? t('register.empty') : t('register.pick')}
              body={all.length === 0 ? t('register.emptyBody') : undefined}
              action={
                all.length === 0 ? (
                  <Button variant="primary" onClick={() => setAdding(true)}>
                    <Plus aria-hidden />
                    {t('register.new')}
                  </Button>
                ) : undefined
              }
            />
          )
        }
      />
      <NewTermDialog open={adding} onOpenChange={setAdding} />
    </Frame>
  )
}

/** Whether a term is found by what was typed: its word, a form, its topic. */
function matches(entry: RegisterEntry, needle: string): boolean {
  return [entry.word, ...entry.forms, entry.topic ?? ''].some((one) =>
    one.toLowerCase().includes(needle),
  )
}

/** The line under a row: the kind, and the topic when there is one. */
function describe(entry: RegisterEntry, t: (key: string) => string): string {
  const kind = t(`register.kinds.${entry.kind}`)
  return entry.topic === null ? kind : `${kind} · ${entry.topic}`
}
