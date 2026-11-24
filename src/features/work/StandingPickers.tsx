import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { Check, ChevronDown, RotateCcw } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Work } from '@/lib/api/types'
import { unpinStatus } from '@/lib/api/works'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { badgeVariantOf } from '@/lib/markIcon'
import { labelOf, say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { chipVariants } from '@/components/ui/chip'
import {
  Menu,
  MenuGroup,
  MenuGroupLabel,
  MenuItem,
  MenuPopup,
  MenuSeparator,
  MenuTrigger,
} from '@/components/ui/menu'
import { useWorkEdit } from '@/features/work/tabs/overview/useWorkEdit'

/*
 * The header's two chips that are also where they are decided (v0.166).
 *
 * What a work is and where it stands were read in the header and changed in
 * the fields widget of the overview - two boxes that repeated the chips above
 * them, on one tab of the card, and on no tab at all for a kind whose
 * overview has no fields widget. The chips are the pickers now, as the
 * collection's has been since v0.92, and the widget keeps the fields.
 */

/** What every picking chip shares with the collection's: it looks pressable. */
const PICKS =
  'target-min cursor-pointer gap-1 hover:border-line-2 hover:text-text focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent'

/** A row of a picking menu, ticked when it is the one that stands. */
function Choice({
  chosen,
  onPick,
  children,
}: {
  chosen: boolean
  onPick: () => void
  children: ReactNode
}) {
  return (
    <MenuItem
      onClick={() => {
        if (!chosen) onPick()
      }}
    >
      <Check aria-hidden className={cn('size-3.5', !chosen && 'invisible')} />
      {children}
    </MenuItem>
  )
}

/** The kind of the work, changed from its chip. */
export function KindPicker({ work }: { work: Work }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const { patch } = useWorkEdit(work)
  const kinds = profile.config.work_kinds

  return (
    <Menu>
      <MenuTrigger
        aria-label={t('work.kindIs', { kind: labelOf(kinds, work.kind) })}
        className={cn(chipVariants({}), PICKS)}
      >
        {labelOf(kinds, work.kind)}
        <ChevronDown aria-hidden className="size-3" />
      </MenuTrigger>
      <MenuPopup align="start">
        <MenuGroup>
          <MenuGroupLabel>{t('work.kind')}</MenuGroupLabel>
          {kinds.map((kind) => (
            <Choice
              key={kind.key}
              chosen={kind.key === work.kind}
              onPick={() => patch.mutate({ kind: kind.key })}
            >
              {sayLabel(kind.label)}
            </Choice>
          ))}
        </MenuGroup>
      </MenuPopup>
    </Menu>
  )
}

/**
 * Where the work stands, and whose word that is. Picking a status pins it -
 * the automation then steps over this work entirely - and the way back is the
 * last row of the same menu.
 */
export function StatusPicker({
  work,
  title,
  children,
}: {
  work: Work
  /** What the chip says on a hover: the publication the status stands on. */
  title?: string
  /** The chip's words: the status, and what it stands on when it does. */
  children: ReactNode
}) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const profile = useProfile()
  const { patch } = useWorkEdit(work)
  const vocabulary = vocabularyOf(profile.config, work.kind)
  const status = vocabulary.statuses.find((s) => s.key === work.status)
  const pinned = work.status_pinned_at != null

  const unpin = useAppMutation({
    mutationFn: () => unpinStatus(work.id),
    failure: 'toast.workSaveFailed',
    refresh: refresh.work,
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
    },
  })

  return (
    <Menu>
      <MenuTrigger
        title={title}
        aria-label={t('work.statusIs', {
          status: status === undefined ? work.status : sayLabel(status.label),
        })}
        className={cn(chipVariants({ variant: badgeVariantOf(status?.colour) }), PICKS)}
      >
        {children}
        <ChevronDown aria-hidden className="size-3" />
      </MenuTrigger>
      <MenuPopup align="start">
        <MenuGroup>
          <MenuGroupLabel>
            {t('work.status')} · {pinned ? t('work.statusPinned') : t('work.statusDerived')}
          </MenuGroupLabel>
          {vocabulary.statuses.map((option) => (
            <Choice
              key={option.key}
              chosen={option.key === work.status}
              onPick={() => patch.mutate({ status: option.key })}
            >
              {sayLabel(option.label)}
            </Choice>
          ))}
        </MenuGroup>
        {pinned && (
          <>
            <MenuSeparator />
            <MenuItem
              disabled={unpin.isPending}
              title={t('work.unpinStatusHint')}
              onClick={() => unpin.mutate()}
            >
              <RotateCcw aria-hidden className="size-3.5" />
              {t('work.unpinStatus')}
            </MenuItem>
          </>
        )}
      </MenuPopup>
    </Menu>
  )
}
