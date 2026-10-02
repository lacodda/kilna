import type { PointerEvent } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { ArrowUpRight, ChevronDown, FolderInput, X } from 'lucide-react'
import type { Bank, BlockView, RegisterEntry } from '@/lib/api/types'
import { BANK } from '@/lib/register'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
  Menu,
  MenuGroup,
  MenuGroupLabel,
  MenuItem,
  MenuPopup,
  MenuSeparator,
  MenuTrigger,
} from '@/components/ui/menu'
import { ReorderGrip } from '@/components/ui/reorderable-list'

interface Props {
  entry: RegisterEntry
  /** The block on show, when the list is one: the word can be taken out of it. */
  block: BlockView | null
  /** Every block of the bank, for where the word is and where it can go. */
  blocks: readonly BlockView[]
  /** It is the word being carried to a block. */
  carrying: boolean
  /** The pointer went down on it: the start of carrying it, if it moves. */
  onGrab: (event: PointerEvent<HTMLElement>) => void
  onBank: (bank: Bank | null) => void
  onPut: (blockId: string) => void
  onTakeOut: () => void
}

/**
 * One word of the bank: the word, how it is sung where it has a way of its
 * own, the blocks it is in, how many works it is sung in, where it stands in
 * the bank, and what can be done to it.
 *
 * "Sung in" is the register's count read off the works' current texts (ADR
 * 0044, ADR 0052) - never stored - so a song rewritten a minute ago already
 * counts. The word is carried to a block by its grip or by itself; the menu
 * behind the folder is the same act for a keyboard.
 */
export function BankWord({
  entry,
  block,
  blocks,
  carrying,
  onGrab,
  onBank,
  onPut,
  onTakeOut,
}: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()

  const holding = blocks.filter((one) => one.term_ids.includes(entry.id))
  // The other blocks it is in, under the word: the one on show goes without
  // saying.
  const elsewhere = holding.filter((one) => one.id !== block?.id).map((one) => one.name)
  const free = blocks.filter((one) => !one.term_ids.includes(entry.id))
  // The ways out of the state it is in: never the one it is already in.
  const moves = BANK.filter((state) => state !== entry.bank)
  const sung = entry.sung.map((one) => one.sung).join(', ')

  return (
    <li
      className={cn(
        'group flex min-w-0 items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-soft',
        entry.bank !== 'fresh' && 'text-dim',
        carrying && 'opacity-50',
      )}
    >
      <ReorderGrip onPointerDown={onGrab} title={t('words.carry')} />
      <span
        className="max-w-60 min-w-0 shrink cursor-grab truncate font-medium active:cursor-grabbing"
        title={entry.word}
        onPointerDown={onGrab}
      >
        {entry.word}
      </span>
      {sung !== '' && (
        <span
          className="max-w-40 min-w-0 shrink truncate font-mono text-xs text-dim"
          title={`${t('words.sungForms')}: ${entry.sung
            .map((one) => `${one.written} → ${one.sung}`)
            .join(', ')}`}
        >
          {sung}
        </span>
      )}
      <span className="min-w-0 flex-1 truncate text-xs text-faint">
        {elsewhere.length > 0
          ? t('words.inBlocks', { count: elsewhere.length, blocks: elsewhere.join(', ') })
          : null}
      </span>
      {entry.uses > 0 && (
        <span className="shrink-0 text-xs text-faint tabular-nums" title={t('words.sungInHint')}>
          {t('words.sungIn', { count: entry.uses })}
        </span>
      )}

      <Menu>
        <MenuTrigger
          render={
            <Button
              size="xs"
              variant="icon"
              className="shrink-0"
              aria-label={`${t('words.stateMenu')}: ${state(entry.bank, t)}`}
              title={t('words.stateMenu')}
            />
          }
        >
          {state(entry.bank, t)}
          <ChevronDown aria-hidden />
        </MenuTrigger>
        <MenuPopup align="end">
          {moves.map((move) => (
            <MenuItem key={move} onClick={() => onBank(move)}>
              {t(`words.states.${move}`)}
            </MenuItem>
          ))}
          {entry.bank !== null && (
            <>
              <MenuSeparator />
              <MenuItem tone="danger" onClick={() => onBank(null)}>
                {t('words.takeOut')}
              </MenuItem>
            </>
          )}
        </MenuPopup>
      </Menu>

      <Menu>
        <MenuTrigger
          render={
            <Button
              size="icon-xs"
              variant="icon"
              title={t('words.putIn')}
              aria-label={t('words.putIn')}
              disabled={blocks.length === 0}
            />
          }
        >
          <FolderInput aria-hidden />
        </MenuTrigger>
        <MenuPopup align="end">
          <MenuGroup>
            <MenuGroupLabel>{t('words.putIn')}</MenuGroupLabel>
            {free.length === 0 ? (
              <MenuItem disabled>{t('words.putNowhere')}</MenuItem>
            ) : (
              free.map((one) => (
                <MenuItem key={one.id} onClick={() => onPut(one.id)}>
                  {one.name}
                </MenuItem>
              ))
            )}
          </MenuGroup>
        </MenuPopup>
      </Menu>
      <Button
        size="icon-xs"
        variant="icon"
        title={t('words.open')}
        aria-label={t('words.open')}
        onClick={() => void navigate(`/register/${entry.id}`)}
      >
        <ArrowUpRight aria-hidden />
      </Button>
      {block !== null && (
        <Button
          size="icon-xs"
          variant="icon"
          title={t('words.removeFromBlock')}
          aria-label={t('words.removeFromBlock')}
          onClick={onTakeOut}
        >
          <X aria-hidden />
        </Button>
      )}
    </li>
  )
}

/** Where a word stands, in the bank's words; a word of a block may be out. */
function state(bank: Bank | null, t: (key: string) => string): string {
  return bank === null ? t('words.notBanked') : t(`words.states.${bank}`)
}
