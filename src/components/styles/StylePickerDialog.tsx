import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { GripVertical, X } from 'lucide-react'
import { fileSrc, listStyleBricks, styleBrickReferences, type StyleBrick } from '@/lib/api'
import { keys } from '@/lib/query'
import { say as sayLabel, styleTypesOf, useProfile } from '@/lib/useProfile'
import { styleIconOf } from '@/lib/styleIcon'
import { useDebounced } from '@/lib/useDebounced'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/ui/AppDialog'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/ui/EmptyState'
import { cn } from '@/lib/utils'

/**
 * Picking the styles an action builds its prompt from.
 *
 * Opened by an action whose template reads `{styles}`, before the run starts:
 * which parts a picture is built from is the question being asked, and it is a
 * different answer every time, so it is not kept on the work.
 *
 * **The order is the answer too.** The picked list is a list, not a set: the
 * first is the spine of the picture and the last is a detail, and the prompt
 * carries them that way. So picked styles show in the order they were picked
 * and can be moved, rather than snapping back into the dictionary's order.
 */
export function StylePickerDialog({
  open,
  onOpenChange,
  onStart,
  actionLabel,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Start the action with these styles, in this order. */
  onStart: (styleBrickIds: string[]) => void
  actionLabel: string
}) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const types = styleTypesOf(config)

  const [picked, setPicked] = useState<string[]>([])
  const [text, setText] = useState('')
  const query = useDebounced(text, 200)

  // Only what can go into a prompt: a draft is unfinished by definition, and a
  // picker quietly full of things nobody has touched is how a dictionary rots.
  const bricks = useQuery({
    queryKey: [...keys.styleBricks, 'ready', query],
    queryFn: () => listStyleBricks({ ready_only: true, query: query || null }),
  })

  const byId = useMemo(
    () => new Map((bricks.data ?? []).map((one) => [one.id, one])),
    [bricks.data],
  )
  const labelOfType = useMemo(
    () => new Map(types.map((one) => [one.key, sayLabel(one.label)])),
    [types],
  )

  const available = (bricks.data ?? []).filter((one) => !picked.includes(one.id))
  const chosen = picked.map((id) => byId.get(id)).filter((one): one is StyleBrick => one !== undefined)

  const move = (from: number, to: number) => {
    if (to < 0 || to >= picked.length) return
    const next = [...picked]
    const [taken] = next.splice(from, 1)
    if (taken === undefined) return
    next.splice(to, 0, taken)
    setPicked(next)
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={actionLabel}
      description={t('styles.pickBody')}
      className="max-w-2xl"
      footer={
        <Button
          variant="primary"
          disabled={picked.length === 0}
          onClick={() => {
            onStart(picked)
          }}
        >
          {t('styles.buildFrom', { count: picked.length })}
        </Button>
      }
    >
      <div className="flex flex-col gap-3">
        {chosen.length > 0 && (
          <ol className="flex flex-col gap-1.5">
            {chosen.map((one, index) => {
              const Icon = styleIconOf(types.find((type) => type.key === one.type_key))
              return (
                <li
                  key={one.id}
                  className="flex items-center gap-2 rounded-lg border border-line bg-raise px-2 py-1.5"
                >
                  <span className="w-4 shrink-0 text-center text-xs text-faint tabular-nums">
                    {index + 1}
                  </span>
                  <Icon aria-hidden className="size-4 shrink-0 text-dim" />
                  <span className="min-w-0 flex-1 truncate text-sm">
                    <span className="text-faint">{labelOfType.get(one.type_key) ?? one.type_key} · </span>
                    {one.name}
                  </span>
                  {/* Buttons rather than a drag handle: two or three parts is
                      the usual number, and a drag surface for three rows is
                      more machinery than the job needs. */}
                  <span className="flex shrink-0 items-center">
                    <Button
                      size="sm"
                      variant="icon"
                      className="h-6 px-1"
                      aria-label={t('styles.moveUp')}
                      disabled={index === 0}
                      onClick={() => move(index, index - 1)}
                    >
                      <GripVertical aria-hidden className="size-3.5 rotate-90" />
                    </Button>
                    <Button
                      size="sm"
                      variant="icon"
                      className="h-6 px-1"
                      aria-label={t('styles.unpick')}
                      onClick={() => setPicked(picked.filter((id) => id !== one.id))}
                    >
                      <X aria-hidden className="size-3.5" />
                    </Button>
                  </span>
                </li>
              )
            })}
          </ol>
        )}

        <Input
          value={text}
          onChange={(event) => setText(event.target.value)}
          placeholder={t('styles.search')}
          aria-label={t('styles.search')}
        />

        {available.length === 0 ? (
          <EmptyState
            title={picked.length > 0 ? t('styles.allPicked') : t('styles.noneReady')}
            body={picked.length > 0 ? undefined : t('styles.noneReadyBody')}
            className="min-h-40"
          />
        ) : (
          <ul className="flex max-h-72 flex-col gap-1 overflow-y-auto">
            {available.map((one) => {
              const Icon = styleIconOf(types.find((type) => type.key === one.type_key))
              return (
                <li key={one.id}>
                  <button
                    type="button"
                    onClick={() => setPicked([...picked, one.id])}
                    className={cn(
                      'flex w-full cursor-pointer items-center gap-2 rounded-lg border border-transparent px-2 py-1.5 text-left',
                      'hover:border-line-2 hover:bg-soft',
                    )}
                  >
                    <Cover brick={one} />
                    <Icon aria-hidden className="size-4 shrink-0 text-dim" />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm">
                        <span className="text-faint">
                          {labelOfType.get(one.type_key) ?? one.type_key} ·{' '}
                        </span>
                        {one.name}
                      </span>
                      <span className="block truncate text-xs text-faint">{one.description}</span>
                    </span>
                  </button>
                </li>
              )
            })}
          </ul>
        )}
      </div>
    </Dialog>
  )
}

/** The first reference, small: a style is recognised by what it looks like. */
function Cover({ brick }: { brick: StyleBrick }) {
  const references = useQuery({
    queryKey: keys.styleReferences(brick.id),
    queryFn: () => styleBrickReferences(brick.id),
    enabled: brick.reference_count > 0,
  })
  const cover = references.data?.[0]
  if (cover === undefined) return <span className="size-8 shrink-0 rounded bg-soft" />
  return (
    <img
      src={fileSrc(cover.path)}
      alt=""
      className="size-8 shrink-0 rounded object-cover"
      draggable={false}
    />
  )
}
