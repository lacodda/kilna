import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { DESTINATIONS } from '@/lib/keys'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogDescription,
  DialogPopup,
  DialogTitle,
} from '@/components/ui/dialog'
import { Kbd } from '@/components/ui/kbd'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** Which nav word names each destination. The chord letters are fixed in
 * `keys.ts`; this only says what to call the screen they reach, so the sheet
 * cannot drift from the shortcuts it documents. */
const SCREEN_NAME: Readonly<Record<string, string>> = Object.freeze({
  '/dashboard': 'nav.dashboard',
  '/catalogue': 'nav.catalogue',
  '/calendar': 'nav.calendar',
  '/journal': 'nav.journal',
  '/trash': 'nav.trash',
  '/settings': 'nav.data',
})

/** One line of the sheet: what the keys are, and what they do. */
function Row({ keys, children }: { keys: string[]; children: string }) {
  return (
    <div className="flex items-baseline gap-3 py-1">
      <Kbd keys={keys} className="shrink-0" />
      <span className="min-w-0 text-sm text-dim">{children}</span>
    </div>
  )
}

/** A heading over a group of shortcuts. */
function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h3 className="pb-1 text-2xs uppercase tracking-caption text-faint">{title}</h3>
      {children}
    </section>
  )
}

/**
 * The list of shortcuts, opened by `?`.
 *
 * Built on dowel's dialog parts rather than on this app's `Dialog`, and that
 * is the difference between a sheet and a decision: `Dialog` ends in Cancel
 * beside an affirmative button, and a reference has neither — there is nothing
 * here to confirm and nothing to take back. Close is the only way out, so it
 * is the only button.
 *
 * The going-places group is written from `DESTINATIONS` rather than typed out,
 * so a chord added in `keys.ts` appears here without anyone remembering to
 * come back. The failure mode of every hand-written shortcut sheet is being a
 * version behind what the application actually answers to.
 */
export function KeyboardSheet({ open, onOpenChange }: Props) {
  const { t } = useTranslation()

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogPopup size="lg">
        <DialogTitle>{t('keys.title')}</DialogTitle>
        <DialogDescription>{t('keys.description')}</DialogDescription>

        <DialogClose
          render={
            <Button
              variant="icon"
              size="icon-sm"
              aria-label={t('dialog.close')}
              className="absolute right-3 top-3"
            />
          }
        >
          <X aria-hidden />
        </DialogClose>

        <div className="mt-4 grid gap-4 sm:grid-cols-2">
          <Group title={t('keys.group.going')}>
            {Object.entries(DESTINATIONS).map(([key, where]) => (
              <Row key={key} keys={['G', key.toUpperCase()]}>
                {t(SCREEN_NAME[where] ?? where)}
              </Row>
            ))}
          </Group>

          <div className="grid gap-4">
            <Group title={t('keys.group.moving')}>
              <Row keys={['Mod', 'K']}>{t('keys.action.search')}</Row>
              <Row keys={['Alt', 'ArrowLeft']}>{t('keys.action.back')}</Row>
              <Row keys={['Alt', 'ArrowRight']}>{t('keys.action.forward')}</Row>
            </Group>

            <Group title={t('keys.group.everywhere')}>
              <Row keys={['?']}>{t('keys.action.help')}</Row>
              <Row keys={['Escape']}>{t('keys.action.close')}</Row>
            </Group>
          </div>
        </div>

        <p className="mt-4 text-xs text-faint">{t('keys.typing')}</p>
      </DialogPopup>
    </Dialog>
  )
}
