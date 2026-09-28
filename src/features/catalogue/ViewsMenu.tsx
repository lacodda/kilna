import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Bookmark, BookmarkPlus, ChevronDown, X } from 'lucide-react'
import { isNarrowed } from '@/lib/catalogue'
import { matchesView, type SavedView, type ViewShape } from '@/lib/views'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { Popover, PopoverPopup, PopoverTitle, PopoverTrigger } from '@/components/ui/popover'

interface Props {
  views: SavedView[]
  shape: ViewShape
  onOpen: (view: SavedView) => void
  onSave: (name: string) => void
  onRemove: (id: string) => void
}

/**
 * The slices worth keeping, and the way to keep one.
 *
 * Behind one button in the toolbar rather than as a row of chips of its own:
 * the mockup's toolbar is two rows, and views were the third. A view is still
 * a question about this screen and nowhere else - the app's rail is where the
 * screens themselves live - so it stays beside the controls it changes.
 *
 * A panel rather than a menu, because keeping a view takes a name, and a
 * field inside a menu fights the menu's own keys. Saving is deliberately a
 * name and a press: a view nobody can tell apart from the next one is a row to
 * scroll past, which is the failure this is meant to prevent.
 */
export function ViewsMenu({ views, shape, onOpen, onSave, onRemove }: Props) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [name, setName] = useState('')

  const active = views.find((view) => matchesView(view, shape))
  // Saving the catalogue as it opens would store "everything, by score"
  // under a name, which is the one slice that needs no shortcut.
  const worthKeeping = isNarrowed(shape.filter) || shape.groupBy !== 'none'

  const commit = () => {
    const trimmed = name.trim()
    if (trimmed === '') return
    onSave(trimmed)
    setName('')
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        render={<Button size="sm" variant={active === undefined ? 'ghost' : 'soft'} />}
      >
        {active === undefined
          ? t('catalogue.views')
          : t('catalogue.viewActive', { name: active.name })}
        <ChevronDown aria-hidden />
      </PopoverTrigger>

      <PopoverPopup size="md" align="start" arrow={false} className="flex flex-col gap-2 p-2">
        <PopoverTitle className="px-1.5 pt-1">
          <span className="caption">{t('catalogue.views')}</span>
        </PopoverTitle>

        {views.length === 0 ? (
          <p className="px-1.5 text-xs text-dim">{t('catalogue.viewsEmpty')}</p>
        ) : (
          <ul className="flex flex-col">
            {/* The way to forget a view sits beside its row rather than inside
                it: a row that opens cannot also hold a cross, or the cross is
                a button inside a button. Opening the view that is on puts it
                back as saved - there is no "off" for a shape the catalogue is
                already in. */}
            {views.map((view) => (
              <li key={view.id} className="flex items-center gap-1">
                <RowButton
                  selected={active?.id === view.id}
                  onClick={() => {
                    onOpen(view)
                    setOpen(false)
                  }}
                  start={<Bookmark aria-hidden className="size-3.5" />}
                >
                  {view.name}
                </RowButton>
                <Button
                  variant="icon"
                  size="icon-sm"
                  onClick={() => onRemove(view.id)}
                  aria-label={t('catalogue.viewRemove', { name: view.name })}
                  title={t('catalogue.viewRemove', { name: view.name })}
                >
                  <X aria-hidden />
                </Button>
              </li>
            ))}
          </ul>
        )}

        <p className="caption border-t border-line px-1.5 pt-2">{t('catalogue.viewSaveCurrent')}</p>
        <form
          className="flex items-center gap-1.5"
          onSubmit={(event) => {
            event.preventDefault()
            commit()
          }}
        >
          <Input
            className="h-control-sm flex-1 text-sm"
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder={t('catalogue.viewNamePlaceholder')}
            aria-label={t('catalogue.viewName')}
            disabled={!worthKeeping}
            title={t('catalogue.viewSaveHint')}
          />
          <Button
            type="submit"
            size="sm"
            variant="primary"
            disabled={!worthKeeping || name.trim() === ''}
          >
            <BookmarkPlus aria-hidden />
            {t('catalogue.viewSave')}
          </Button>
        </form>
      </PopoverPopup>
    </Popover>
  )
}
