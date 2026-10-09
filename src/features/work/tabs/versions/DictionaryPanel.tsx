import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Composition } from '@/lib/api/types'
import { brickMatches, offeredBricks } from '@/lib/phrases'
import { queries } from '@/lib/query/queries'
import { styleIconOf } from '@/lib/styleIcon'
import { useDebounced } from '@/lib/useDebounced'
import { say as sayLabel, styleTypesOf, useProfile } from '@/lib/useProfile'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { Scroll } from '@/components/frame'

interface Props {
  composition: Composition
  /** Put the phrase into the text, at the caret. */
  onPick: (phrase: string) => void
}

/**
 * The dictionary beside a style text (v0.94): the composition's blocks with
 * their phrases, each with what it means, searchable in any language - the
 * phrase, its name, or the explanation. A press puts the phrase into the
 * text where the caret is, so a style is written knowing what each phrase
 * will do.
 */
export function DictionaryPanel({ composition, onPick }: Props) {
  const { t } = useTranslation()
  const types = styleTypesOf(useProfile().config)
  const [text, setText] = useState('')
  const query = useDebounced(text, 150)
  const bricks = useQuery(queries.styleBricksMatching(null, ''))
  const house = useQuery(queries.houseStyles())
  const houseSet = useMemo(() => new Set(house.data ?? []), [house.data])

  return (
    <aside
      aria-label={t('phrases.dictionary')}
      className="flex w-72 shrink-0 flex-col gap-2 border-l border-line p-2"
    >
      <Input
        value={text}
        onChange={(event) => setText(event.target.value)}
        placeholder={t('phrases.search')}
        aria-label={t('phrases.search')}
      />
      <Scroll label={t('phrases.dictionary')} contentClassName="flex flex-col gap-3 pr-1">
        {composition.parts.map((part) => {
          const type = types.find((one) => one.key === part.type)
          const Icon = styleIconOf(type)
          const offered = offeredBricks(bricks.data ?? [], part.type, houseSet).filter((brick) =>
            brickMatches(brick, query),
          )
          if (offered.length === 0) return null
          return (
            <section key={part.type} className="flex flex-col gap-0.5">
              <h4 className="caption flex items-center gap-1.5 px-1">
                <Icon aria-hidden className="size-3" />
                {type === undefined ? part.type : sayLabel(type.label)}
              </h4>
              {offered.map((brick) => (
                <RowButton
                  key={brick.id}
                  onClick={() => onPick(brick.description ?? brick.name)}
                  description={sayLabel(brick.explanation) || undefined}
                  title={sayLabel(brick.explanation) || t('phrases.insert')}
                >
                  <span className="font-mono">{brick.description ?? brick.name}</span>
                </RowButton>
              ))}
            </section>
          )
        })}
      </Scroll>
    </aside>
  )
}
