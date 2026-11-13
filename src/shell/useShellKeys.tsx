import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { ArrowLeft, ArrowRight, Undo2 } from 'lucide-react'
import { useCommands } from '@/components/ui/commands'
import { useUndo } from '@/lib/useUndo'

/**
 * The keys the shell answers before the workspace is read: walking the
 * history, taking back the last change, and Escape.
 *
 * Declared on dowel's list of commands (wish 4308), which the keyboard, the
 * palette and the sheet of shortcuts all read - so a key is written once and
 * none of them can be a version behind. Declared by the shell before anything
 * inside it, so a screen's own keys never take these.
 *
 * - Alt+arrows are the browser's own back and forward, and kilna is routed
 *   like a browser (ADR 0006). They work while typing: Alt is not a
 *   character, so nothing is interrupted.
 * - Mod+Z takes back the last thing that changed the workspace - never inside
 *   a field, where it is the field's own undo: stealing it would take back a
 *   saved edit while the person meant the word they just typed. Shift is not
 *   ignored: Mod+Shift+Z is redo, and kilna has none.
 * - Escape closes what is open. Declared without a run: the dialog or the
 *   menu that is open answers it, and it is here to be listed.
 */
export function useShellKeys(): void {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { undo } = useUndo()
  const moving = t('keys.group.moving')
  const actions = t('search.group.action')

  useCommands([
    {
      id: 'back',
      label: t('keys.action.back'),
      group: moving,
      icon: <ArrowLeft aria-hidden />,
      keys: 'Alt+ArrowLeft',
      whileTyping: true,
      run: () => navigate(-1),
    },
    {
      id: 'forward',
      label: t('keys.action.forward'),
      group: moving,
      icon: <ArrowRight aria-hidden />,
      keys: 'Alt+ArrowRight',
      whileTyping: true,
      run: () => navigate(1),
    },
    {
      id: 'undo',
      label: t('keys.action.undo'),
      group: actions,
      icon: <Undo2 aria-hidden />,
      keys: 'Mod+Z',
      run: undo,
    },
    { id: 'close', label: t('keys.action.close'), group: actions, keys: 'Escape' },
  ])
}
