import { useEffect, type RefObject } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open as openFile } from '@tauri-apps/plugin-dialog'
import { ImagePlus, X } from 'lucide-react'
import { attachAsset, detachAsset, fileSrc } from '@/lib/api/assets'
import { pasteStyleReference } from '@/lib/api/styles'
import { picturesAmong } from '@/lib/drop'
import { PICTURES } from '@/lib/media'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { Skeleton } from '@/components/ui/skeleton'
import { Loaded } from '@/components/Loaded'
import { pictureIn, readPicture, type PastedPicture } from '@/features/styles/pastedPicture'
import { useWindowDrop } from '@/features/styles/useWindowDrop'

interface Props {
  brickId: string
  /**
   * What a drop lands on to become a reference: the whole open style, so a
   * picture let go anywhere on it arrives, not only over this strip - which
   * may be scrolled out of view under a long description.
   */
  zone: RefObject<HTMLElement | null>
}

/**
 * The pictures a style is described from, and the three ways they come in.
 *
 * The same three a scene's frames have, for the same reasons: a file on disk
 * through the picker, a file from a folder dropped on the style, and a
 * picture looked at in a generator's tab pasted with Ctrl+V. Until v0.79 only
 * the picker and the paste worked here, and the hint promised the drop.
 */
export function StyleReferences({ brickId, zone }: Props) {
  const { t } = useTranslation()
  const references = useQuery(queries.styleReferences(brickId))

  const attach = useAppMutation({
    mutationFn: async (paths: string[]) => {
      for (const path of paths) await attachAsset(path, { style_brick_id: brickId })
    },
    failure: 'styles.referenceFailed',
    refresh: refresh.style,
  })

  const paste = useAppMutation({
    mutationFn: ({ bytes, name }: PastedPicture) => pasteStyleReference(brickId, bytes, name),
    failure: 'styles.referenceFailed',
    refresh: refresh.style,
  })
  const pasteMutate = paste.mutate

  const detach = useAppMutation({
    mutationFn: (assetId: string) => detachAsset(assetId),
    refresh: refresh.style,
  })

  const add = (paths: string[]) => {
    const { pictures, others } = picturesAmong(paths)
    if (others > 0) say.warn(t('styles.notPictures', { count: others }))
    if (pictures.length > 0) attach.mutate(pictures)
  }

  const over = useWindowDrop(zone, add)

  // A picture pasted while the style is open is the style's: a paste has no
  // position, and the open style is where the person is working. The
  // listener lives as long as the style is open, so Ctrl+V elsewhere keeps
  // its usual meaning.
  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      const file = pictureIn(event)
      if (file === undefined) return
      event.preventDefault()
      readPicture(file).then(pasteMutate, (cause: unknown) => say.failed(cause))
    }
    window.addEventListener('paste', onPaste)
    return () => window.removeEventListener('paste', onPaste)
  }, [pasteMutate])

  const choose = async () => {
    const picked = await openFile({
      multiple: true,
      filters: [{ name: t('styles.pictures'), extensions: PICTURES }],
    })
    if (picked === null) return
    add(Array.isArray(picked) ? picked : [picked])
  }

  return (
    // A group, named by its caption: a grid of pictures has no one control a
    // label could point at, and a label around it once forwarded a click on
    // the caption to the first picture's remove cross.
    <FieldGroup label={t('styles.referenceList')} help={t('styles.referenceHint')}>
      <div
        // Read by the open style around it, which draws its own outline
        // while a drag is over any part of it.
        data-dropping={over ? '' : undefined}
        className={cn(
          'flex flex-col gap-2 rounded-lg border border-dashed p-2 transition-colors',
          over ? 'border-accent bg-accent-soft' : 'border-line',
        )}
      >
        <Loaded query={references} plain skeleton={<Skeleton className="size-24" />}>
          {(assets) =>
            assets.length === 0 ? null : (
              <ul className="flex flex-wrap gap-2">
                {assets.map((asset) => (
                  <li key={asset.id} className="group relative">
                    <img
                      src={fileSrc(asset.path)}
                      alt={asset.original_name ?? ''}
                      className="size-24 rounded-md object-cover"
                      draggable={false}
                    />
                    {/* The ground under the cross is this corner's, not the
                        button's: over a picture a bare glyph is lost. Shown
                        on hover, and whenever the cross has the keyboard. */}
                    <span className="absolute -top-1.5 -right-1.5 rounded-md border border-line bg-raise opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
                      <Button
                        variant="icon"
                        size="icon-xs"
                        onClick={() => detach.mutate(asset.id)}
                        aria-label={t('styles.removeReference')}
                        title={t('styles.removeReference')}
                      >
                        <X aria-hidden />
                      </Button>
                    </span>
                  </li>
                ))}
              </ul>
            )
          }
        </Loaded>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => void choose()}
            disabled={attach.isPending || paste.isPending}
          >
            <ImagePlus aria-hidden />
            {t('styles.addReference')}
          </Button>
          {over && <span className="text-xs text-accent">{t('styles.dropToAdd')}</span>}
        </div>
      </div>
    </FieldGroup>
  )
}
