import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { BookPlus, Sparkles } from 'lucide-react'
import type { Composition, TextCheck } from '@/lib/api/types'
import { say as sayLabel, styleTypesOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import { AddPhraseDialog } from '@/features/styles/AddPhraseDialog'
import { useExplainPhrases } from '@/features/styles/useExplainPhrases'

interface Props {
  check: TextCheck
  composition: Composition
}

/**
 * What the dictionary says of a style text (v0.94), under its toolbar: the
 * phrases it knows - each with what it means, a hover away - and the tags it
 * does not, each one press from the dictionary: kept by hand, or explained
 * by the assistant. "Explain all" sends every unknown tag at once.
 */
export function PhraseStrip({ check, composition }: Props) {
  const { t } = useTranslation()
  const types = styleTypesOf(useProfile().config)
  const explain = useExplainPhrases(composition.key)
  const [adding, setAdding] = useState<string | null>(null)

  if (check.phrases.length === 0 && check.unknown.length === 0) return null
  const typeLabel = (key: string) => {
    const type = types.find((one) => one.key === key)
    return type === undefined ? key : sayLabel(type.label)
  }

  return (
    <div className="flex shrink-0 flex-col gap-1 border-b border-line px-3 py-1.5 text-xs text-dim">
      {check.phrases.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="mr-1 font-medium">
            {t('phrases.known', { count: check.phrases.length })}
          </span>
          {check.phrases.map((hit) => (
            <span
              key={hit.brick_id}
              title={[typeLabel(hit.type_key), sayLabel(hit.explanation), hit.when ?? '']
                .filter((part) => part !== '')
                .join(' · ')}
              className="phrase-known rounded-sm px-1.5 py-px font-mono"
            >
              {hit.phrase}
              {hit.count > 1 && <span className="ml-1 tabular-nums">×{hit.count}</span>}
            </span>
          ))}
        </div>
      )}
      {check.unknown.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="mr-1 font-medium text-warn">
            {t('phrases.unknown', { count: check.unknown.length })}
          </span>
          {check.unknown.map((unknown) => (
            <Menu key={unknown.phrase}>
              <MenuTrigger
                render={
                  <Button
                    variant="ghost"
                    size="xs"
                    className="phrase-unknown-chip font-mono"
                    title={t('phrases.unknownHint')}
                  />
                }
              >
                {unknown.phrase}
              </MenuTrigger>
              <MenuPopup align="start">
                <MenuItem onClick={() => setAdding(unknown.phrase)}>
                  <BookPlus aria-hidden className="size-3.5" />
                  {t('phrases.addByHand')}
                </MenuItem>
                {explain.label !== undefined && (
                  <MenuItem
                    disabled={explain.pending}
                    onClick={() =>
                      explain.explain([{ phrase: unknown.phrase, count: unknown.count }])
                    }
                  >
                    <Sparkles aria-hidden className="size-3.5" />
                    {explain.label}
                  </MenuItem>
                )}
              </MenuPopup>
            </Menu>
          ))}
          {explain.label !== undefined && check.unknown.length > 1 && (
            <Button
              variant="soft"
              size="xs"
              disabled={explain.pending}
              onClick={() =>
                explain.explain(
                  check.unknown.map((one) => ({ phrase: one.phrase, count: one.count })),
                )
              }
            >
              <Sparkles aria-hidden />
              {t('phrases.explainAll', { label: explain.label })}
            </Button>
          )}
        </div>
      )}
      {adding !== null && (
        <AddPhraseDialog
          open
          onOpenChange={(open) => {
            if (!open) setAdding(null)
          }}
          composition={composition}
          phrase={adding}
        />
      )}
    </div>
  )
}
