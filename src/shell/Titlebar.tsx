import { useCallback, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { FileText, PanelLeftClose, PanelLeftOpen, Plus, Search } from 'lucide-react'
import { screenAt } from '@/app/screens'
import { queries } from '@/lib/query/queries'
import { openWorkId } from '@/lib/route'
import { useProfile } from '@/lib/useProfile'
import { Breadcrumbs } from '@/components/ui/breadcrumbs'
import { Button } from '@/components/ui/button'
import { Kbd } from '@/components/ui/kbd'
import { ProductMark } from '@/components/ui/product-mark'
import { useShortcut } from '@/components/ui/shortcut'
import { TitleBar } from '@/components/ui/window-frame'
import { NewWorkDialog } from '@/components/NewWorkDialog'
import { AssistantButton } from '@/features/assistant/AssistantDrawer'
import { Bell } from '@/shell/Bell'
import { CommandPalette } from '@/shell/CommandPalette'

interface Props {
  works: number
  /** Whether the main menu is folded to icons: the handle says which way it
   *  will go. */
  compact: boolean
  onToggleRail: () => void
  /** Open the list of shortcuts - the palette offers it among its actions. */
  onShowKeys: () => void
}

/**
 * Where you are, as a trail rather than a word.
 *
 * On an open card the screen's own name is not enough: "Works" says nothing
 * about which work, and the way back to the list is otherwise the browser's
 * back button alone. The first crumb is a link; the last is where you stand.
 */
function Trail() {
  const { t } = useTranslation()
  const location = useLocation()
  // Read from the path rather than `useParams`: the title bar is a sibling of
  // `<Routes>`, not a descendant, so it matches no route and would always see
  // an empty params object.
  const workId = openWorkId(location.pathname)

  const work = useQuery({ ...queries.work(workId ?? ''), enabled: workId !== undefined })

  const screen = t(screenAt(location.pathname).nav)
  const items =
    workId === undefined
      ? [{ id: 'screen', label: screen }]
      : // Nothing while the title loads, rather than a placeholder that is
        // replaced a moment later - the trail would jump under the cursor.
        [
          { id: 'catalogue', label: screen },
          { id: 'work', label: work.data?.title ?? '' },
        ]

  return (
    <Breadcrumbs label={t('shell.trail')} items={items} render={() => <Link to="/catalogue" />} />
  )
}

/** The mark and the name, where a system title bar would print them.
 *
 * The mark is the line's own, drawn by dowel at the level its size calls for,
 * the same tile every product of the line wears in its title bar. On a narrow
 * window the name and the version go and the mark stays: the trail beside it
 * is what says where you are, and it needs the room more. Narrow is under
 * 1000px rather than the mockup's 900: kilna's search keeps 16rem where the
 * mockup's gave down to 150px, and at the window's narrowest, 900, the trail
 * had no room left for the title of the open work. The count of works in the
 * bar goes at the same width, for the same reason. */
function Brand() {
  const { t } = useTranslation()
  return (
    <div className="flex shrink-0 items-center gap-2 pr-1.5">
      <ProductMark product="kilna" size={20} />
      <b className="text-base font-semibold max-[1000px]:hidden">{t('app.name')}</b>
      <small className="font-mono text-2xs text-faint max-[1000px]:hidden">{__APP_VERSION__}</small>
    </div>
  )
}

/**
 * The handle that folds the main menu to icons and back.
 *
 * In the title bar rather than at the foot of the menu: it sits above the
 * rail it changes, where the menu button of every desktop application is,
 * and it stays in the same place whichever width the rail has. */
function RailHandle({ compact, onToggle }: { compact: boolean; onToggle: () => void }) {
  const { t } = useTranslation()
  const label = t(compact ? 'shell.expandMenu' : 'shell.collapseMenu')
  const Icon = compact ? PanelLeftOpen : PanelLeftClose
  return (
    <Button
      variant="icon"
      size="icon-sm"
      title={label}
      aria-label={label}
      aria-expanded={!compact}
      onClick={onToggle}
    >
      <Icon aria-hidden />
    </Button>
  )
}

/**
 * The New button, beside the bell.
 *
 * One press, straight to the dialog. This was a labelled button with a
 * dropdown of the kinds, and it asked the question twice: the menu named Song,
 * Instrumental, Video, Short - and then the dialog opened with a Kind field
 * offering the same four, because a kind pressed by mistake must not cost the
 * dialog. So the menu only delayed the box where the title is typed, which is
 * the thing actually being added. The dialog opens on the first kind and the
 * field inside changes it.
 *
 * A page with a plus over it, the way the bell beside it carries its count:
 * the shape says what is made, the plus says a new one.
 */
function NewButton({ onPress }: { onPress: () => void }) {
  const { t } = useTranslation()
  return (
    <Button
      variant="icon"
      size="icon-sm"
      className="relative"
      title={t('shell.new')}
      aria-label={t('shell.new')}
      onClick={onPress}
    >
      <FileText aria-hidden />
      <Plus
        aria-hidden
        className="absolute -right-0.5 -top-0.5 size-2.5 rounded-full bg-raise text-accent"
      />
    </Button>
  )
}

/**
 * The window's title bar, and the application's.
 *
 * There is one bar rather than a system one over an application one: the
 * menu handle, the mark, the trail, the search in the middle, the count, the
 * New button, the assistant and the bell, then the window's own buttons - the
 * strip scheda draws, with this application's things in it. It is dowel's
 * TitleBar, which makes everything not a control a handle to drag the window
 * by and holds the search at the window's centre, whatever the trail beside
 * it says.
 */
export function Titlebar({ works, compact, onToggleRail, onShowKeys }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { config } = useProfile()
  const [searching, setSearching] = useState(false)

  // The dialog behind the New button, lifted to the bar so the palette's
  // "New work" opens the same one.
  const [kind, setKind] = useState<string | null>(null)
  const newWork = () => setKind(config.work_kinds[0]?.key ?? null)

  // Ctrl+K anywhere, including from inside a text field: the palette is a way
  // out of wherever you are, not a control that belongs to one screen. Bound
  // and drawn from the same `['Mod', 'K']`, so the key on the button and the
  // key that works cannot drift apart.
  const openPalette = useCallback(() => setSearching(true), [])
  useShortcut(['Mod', 'K'], openPalette, { whileTyping: true })

  return (
    <>
      <TitleBar
        // The middle column is kilna's own, 16 to 22rem, rather than the
        // primitive's `auto`: a search sized by its words would be as narrow
        // as its placeholder. The right one never goes below what it holds:
        // at the window's narrowest, 900px, an even split cut the close
        // button in half, and the trail on the left is the side that can
        // give - it truncates. And the bar is the page's ground, as in the
        // mockup, rather than a raised strip.
        className="grid-cols-[minmax(0,1fr)_minmax(16rem,22rem)_minmax(min-content,1fr)] bg-bg"
        labels={{
          minimize: t('shell.minimize'),
          maximize: t('shell.maximize'),
          restore: t('shell.restore'),
          close: t('shell.close'),
        }}
        center={
          // A field to look at and a button to press: it opens the palette,
          // which is where the typing happens.
          <Button
            variant="ghost"
            size="sm"
            onClick={openPalette}
            className="w-full min-w-0 justify-start text-faint"
          >
            <Search aria-hidden />
            <span className="truncate">{t('search.placeholder')}</span>
            <Kbd keys={['Mod', 'K']} aria-hidden className="ml-auto" />
          </Button>
        }
        actions={
          <>
            <p className="pr-1.5 text-sm whitespace-nowrap text-dim max-[1000px]:hidden">
              {t('status.works')} <span className="font-mono text-text">{works}</span>
            </p>
            <NewButton onPress={newWork} />
            <AssistantButton />
            <Bell />
            <span aria-hidden className="mx-0.5 h-4.5 w-px bg-line" />
          </>
        }
      >
        <div className="flex min-w-0 items-center gap-2 pl-1.5">
          <RailHandle compact={compact} onToggle={onToggleRail} />
          <Brand />
          <Trail />
        </div>
      </TitleBar>

      {/* Beside the bar rather than inside it: both are drawn in a portal,
          but React bubbles a press in a portal to the component it was
          declared in, and inside the bar a drag across the dimmed page behind
          either reached the bar's own handler - which moves the window. */}
      <NewWorkDialog
        kind={kind}
        onClose={() => setKind(null)}
        onCreated={(workId) => navigate(`/works/${workId}`)}
      />
      <CommandPalette
        open={searching}
        onOpenChange={setSearching}
        shell={{ newWork, showKeys: onShowKeys, toggleRail: onToggleRail, compact }}
      />
    </>
  )
}
