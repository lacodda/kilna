import type { KeyboardEvent } from 'react'
import { useTranslation } from 'react-i18next'
import type { CardView, Relation } from '@/lib/api/types'
import { cardKindsOf } from '@/lib/canon'
import { CanonGlyph } from '@/lib/canonIcon'
import { useProfile } from '@/lib/useProfile'
import { clipped, ring, toneOf } from '@/features/canon/graph'

const WIDTH = 264
const HEIGHT = 208
const RADIUS = 76

/**
 * The card among the cards it is related to (v0.143, FR-18 18.4): itself in
 * the middle, the others on a ring, each line with the first words of its
 * relation. A node wears its kind's glyph and colour; pressing one opens it,
 * and its tooltip says the whole relation and who may know of it.
 */
export function MiniGraph({ view, onOpen }: { view: CardView; onOpen: (id: string) => void }) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const kinds = cardKindsOf(config)
  // One node a card, however many relations stand between the two.
  const others: { id: string; relations: Relation[] }[] = []
  for (const relation of view.relations) {
    const held = others.find((one) => one.id === relation.other_id)
    if (held === undefined) others.push({ id: relation.other_id, relations: [relation] })
    else held.relations.push(relation)
  }
  const centre = { x: WIDTH / 2, y: HEIGHT / 2 }
  const spots = ring(others.length, RADIUS, centre)
  const open = (id: string) => (event: KeyboardEvent) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      onOpen(id)
    }
  }

  return (
    <svg
      viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      className="w-full"
      role="group"
      aria-label={t('canon.graphOf', { title: view.card.title ?? t('canon.untitled') })}
    >
      {others.map((other, at) => {
        const spot = spots[at]!
        const words = other.relations
          .map((relation) => relation.label)
          .filter((label): label is string => label !== null && label !== '')
          .join(' · ')
        return (
          <g key={`line-${other.id}`}>
            <line
              x1={centre.x}
              y1={centre.y}
              x2={spot.x}
              y2={spot.y}
              className="stroke-line-2"
              strokeWidth={1}
            />
            {words !== '' && (
              <text
                x={(centre.x + spot.x) / 2}
                y={(centre.y + spot.y) / 2 - 3}
                textAnchor="middle"
                className="fill-faint text-2xs"
              >
                {clipped(words, 16)}
              </text>
            )}
          </g>
        )
      })}
      <g>
        <circle
          cx={centre.x}
          cy={centre.y}
          r={17}
          strokeWidth={2}
          className={toneOf(view.card.kind, kinds)}
        />
        <CanonGlyph
          kind={config.note_kinds?.find((kind) => kind.key === view.card.kind)}
          x={centre.x - 8}
          y={centre.y - 8}
          width={16}
          height={16}
          className="text-text"
        />
      </g>
      {others.map((other, at) => {
        const spot = spots[at]!
        const first = other.relations[0]!
        const kind = config.note_kinds?.find((one) => one.key === first.other_kind)
        const title = first.other_title ?? t('canon.untitled')
        const said = other.relations
          .map((relation) =>
            [relation.label, t(`canon.layer.long.${relation.link.layer}`)]
              .filter((part) => part !== null && part !== '')
              .join(' · '),
          )
          .join('; ')
        return (
          <g
            key={other.id}
            role="link"
            tabIndex={0}
            aria-label={`${title} · ${said}`}
            className="cursor-pointer outline-none focus-visible:[&>circle]:stroke-[3]"
            onClick={() => onOpen(other.id)}
            onKeyDown={open(other.id)}
          >
            <title>{`${title} · ${said}`}</title>
            <circle
              cx={spot.x}
              cy={spot.y}
              r={13}
              strokeWidth={1.5}
              className={toneOf(first.other_kind, kinds)}
            />
            <CanonGlyph
              kind={kind}
              x={spot.x - 7}
              y={spot.y - 7}
              width={14}
              height={14}
              className="text-text"
            />
            <text x={spot.x} y={spot.y + 24} textAnchor="middle" className="fill-dim text-2xs">
              {clipped(title, 14)}
            </text>
          </g>
        )
      })}
    </svg>
  )
}
