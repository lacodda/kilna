import { Toaster as Sonner } from 'sonner'

/**
 * Toasts, dressed in the app's tokens.
 *
 * Sonner's own light/dark themes are bypassed entirely: our theme has three
 * states (system, light, dark) resolved in CSS, so telling sonner which one is
 * active would mean tracking it twice and getting it wrong once.
 */
export function Toaster() {
  return (
    <Sonner
      position="bottom-right"
      // Long enough to read a sentence and reach an Undo button.
      duration={5000}
      gap={8}
      toastOptions={{
        unstyled: true,
        classNames: {
          toast:
            'flex w-full items-start gap-2.5 rounded-xl border border-line bg-raise p-3.5 text-sm text-text shadow-raise',
          title: 'font-medium',
          description: 'mt-0.5 text-xs text-dim',
          actionButton:
            'ml-auto shrink-0 cursor-pointer rounded-[9px] bg-accent-soft px-2.5 py-1 text-xs font-medium text-accent-2 hover:bg-accent/25',
          cancelButton:
            'ml-auto shrink-0 cursor-pointer rounded-[9px] px-2.5 py-1 text-xs text-dim hover:bg-soft',
          closeButton: 'text-dim hover:text-text',
          error: 'border-bad/40',
          success: 'border-good/40',
          warning: 'border-warn/40',
          icon: 'shrink-0',
        },
      }}
    />
  )
}
