import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ChevronsUpDown, Plus, Settings2 } from 'lucide-react'
import { queries } from '@/lib/query/queries'
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
import { NewSpaceDialog } from '@/features/spaces/NewSpaceDialog'
import { SpaceGlyph } from '@/features/spaces/look'
import { useOpenSpace } from '@/features/spaces/useOpenSpace'

interface Props {
  /** Icons only: the name and the chevron are dropped. */
  compact: boolean
}

/**
 * The spaces this machine knows, at the foot of the menu: the open one on the
 * button, the others - the newest opened first - one press away (ADR 0077).
 * Until v0.109 the same place switched profiles inside one database; a space
 * is a database of its own, so switching opens another without a restart.
 */
export function SpaceSwitcher({ compact }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const current = useQuery(queries.currentSpace())
  const spaces = useQuery(queries.spaces())
  const open = useOpenSpace()
  const [making, setMaking] = useState(false)

  if (current.data === undefined) return null
  const here = current.data.space
  const others = (spaces.data ?? []).filter(
    (space) => !space.open && space.trashed_at === null && space.reachable,
  )

  return (
    <>
      <Menu>
        <MenuTrigger
          render={
            <Button
              variant="ghost"
              className={compact ? 'size-9 justify-center p-0' : 'w-full justify-start gap-2 px-2'}
              aria-label={t('spaces.switch', { name: here.name })}
              title={compact ? here.name : undefined}
            />
          }
        >
          <SpaceGlyph space={here} />
          {!compact && (
            <>
              <span className="min-w-0 flex-1 truncate text-left text-text">{here.name}</span>
              <ChevronsUpDown aria-hidden className="size-3.5 text-dim" />
            </>
          )}
        </MenuTrigger>
        <MenuPopup align="start" side="top">
          {others.length > 0 && (
            <MenuGroup>
              <MenuGroupLabel>{t('spaces.recent')}</MenuGroupLabel>
              {others.map((space) => (
                <MenuItem
                  key={space.id}
                  disabled={open.isPending}
                  onClick={() => open.mutate(space.id)}
                >
                  <SpaceGlyph space={space} />
                  <span className="min-w-0 flex-1 truncate">{space.name}</span>
                  <span className="text-dim">
                    {space.kind !== null ? t(`spaces.kinds.${space.kind}`) : ''}
                  </span>
                </MenuItem>
              ))}
            </MenuGroup>
          )}
          {others.length > 0 && <MenuSeparator />}
          <MenuItem onClick={() => setMaking(true)}>
            <Plus aria-hidden />
            {t('spaces.new')}
          </MenuItem>
          <MenuItem onClick={() => navigate('/settings/spaces')}>
            <Settings2 aria-hidden />
            {t('spaces.manage')}
          </MenuItem>
        </MenuPopup>
      </Menu>
      <NewSpaceDialog open={making} onOpenChange={setMaking} />
    </>
  )
}
