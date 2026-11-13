import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Kbd, keyLabel } from '@/components/ui/kbd'
import { ShortcutsDialog } from '@/components/ui/shortcuts-dialog'

/**
 * The list of shortcuts, opened by `?`: dowel's ShortcutsDialog, written from
 * the list of commands rather than by hand (wish 4308).
 *
 * It lists what the shell and the open screen have declared, under the
 * headings they give themselves, with the keys they answer to now - so it
 * cannot promise a key that does not work, the failure mode of every
 * hand-written sheet. It declares its own command, so `?` opens it, the
 * palette offers it, and it lists itself.
 *
 * What keys cannot say is drawn after the list: the stress gesture of a sung
 * text (ADR 0053), which is a press of the pointer with a modifier held, and
 * the rule that letter shortcuts stand aside while someone types.
 */
export function KeyboardSheet() {
  const { t } = useTranslation()
  const heading = useId()

  return (
    <ShortcutsDialog
      title={t('keys.title')}
      closeLabel={t('dialog.close')}
      description={t('keys.description')}
      group={t('search.group.action')}
    >
      {/* On a vowel under the pointer, or at the caret - the same key on a
          Cyrillic layout, where it types э. The editor answers both itself:
          they belong to the text being written, not to the window. */}
      <div role="group" aria-labelledby={heading} className="mt-4">
        <h3 id={heading} className="caption pb-1">
          {t('keys.group.sung')}
        </h3>
        <dl>
          <div className="flex items-baseline justify-between gap-3 py-1">
            <dt className="min-w-0 text-sm">{t('keys.action.stressVowel')}</dt>
            {/* A press of the pointer is no key the notation can name: the
                modifier held and the press are caps of their own. */}
            <dd className="flex shrink-0 items-center gap-0.5">
              <Kbd>{keyLabel('Alt')}</Kbd>
              <Kbd>{t('keys.click')}</Kbd>
            </dd>
          </div>
          <div className="flex items-baseline justify-between gap-3 py-1">
            <dt className="min-w-0 text-sm">{t('keys.action.stressCaret')}</dt>
            <dd className="flex shrink-0">
              <Kbd keys="Alt+'" />
            </dd>
          </div>
        </dl>
      </div>

      <p className="mt-4 text-xs text-faint">{t('keys.typing')}</p>
    </ShortcutsDialog>
  )
}
