import { useTranslation } from 'react-i18next'
import { fileSrc } from '@/lib/api/assets'
import type { Asset } from '@/lib/api/types'
import { RowMenu } from '@/components/RowMenu'
import { cn } from '@/lib/utils'

/** The roles a picture of a card can have; the backend holds the same six. */
const PICTURE_ROLES = ['portrait', 'reference', 'outfit', 'mood', 'still', 'mark'] as const

/**
 * One picture of a card or of one of its facts, with its role and what can be
 * done to it: given another role, or taken off.
 */
export function CardPicture({
  picture,
  onRole,
  onRemove,
  small = false,
}: {
  picture: Asset
  onRole: (role: string) => void
  onRemove: () => void
  /** Drawn beside a fact rather than in the card's gallery. */
  small?: boolean
}) {
  const { t } = useTranslation()
  const role = t(`canon.role.${picture.kind}`, { defaultValue: picture.kind })
  return (
    <figure
      className={cn(
        'group relative shrink-0 overflow-hidden bg-soft',
        small ? 'size-12 rounded-md' : 'aspect-square rounded-lg',
      )}
      title={small ? `${role} · ${picture.original_name ?? ''}` : undefined}
    >
      <img
        src={fileSrc(picture.path)}
        alt={picture.original_name ?? ''}
        className="size-full object-cover"
        draggable={false}
      />
      {!small && (
        <figcaption className="absolute inset-x-0 bottom-0 bg-bg/75 px-1 font-mono text-2xs text-text">
          {role}
        </figcaption>
      )}
      <div className="absolute top-0.5 right-0.5 opacity-0 group-focus-within:opacity-100 group-hover:opacity-100">
        <RowMenu
          label={t('canon.pictureActions')}
          actions={[
            ...PICTURE_ROLES.filter((one) => one !== picture.kind).map((one) => ({
              key: one,
              label: t('canon.makeRole', { role: t(`canon.role.${one}`) }),
              onSelect: () => onRole(one),
            })),
            { key: 'remove', label: t('canon.removePicture'), danger: true, onSelect: onRemove },
          ]}
        />
      </div>
    </figure>
  )
}
