import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Toast,
  ToastAction,
  ToastClose,
  ToastDescription,
  ToastProvider,
  ToastTitle,
  ToastViewport,
  useToastManager,
} from '@/components/ui/toast'
import { toastManager } from '@/lib/toast'

/**
 * Toasts, dressed in the app's tokens.
 *
 * The stripe, the colours and the enter and leave belong to the registry
 * component; what is here is which parts a kilna toast has. The action button
 * is rendered only when a toast carries one, because `undoable` and
 * `withAction` are the only two of the seven that do.
 */
function ToastList() {
  const { t } = useTranslation()
  const { toasts } = useToastManager()

  return toasts.map((toast) => (
    <Toast key={toast.id} toast={toast}>
      <ToastTitle />
      {toast.description !== undefined && <ToastDescription />}
      {toast.actionProps !== undefined && <ToastAction />}
      <ToastClose aria-label={t('dialog.close')} />
    </Toast>
  ))
}

/**
 * Wraps the app so anything inside can raise a toast, and provides the corner
 * they stack in.
 *
 * The manager is the one `say` writes to, passed in rather than created here:
 * a provider that made its own would leave every toast raised outside React
 * going nowhere.
 */
export function Toaster({ children }: { children: ReactNode }) {
  return (
    <ToastProvider toastManager={toastManager}>
      {children}
      <ToastViewport>
        <ToastList />
      </ToastViewport>
    </ToastProvider>
  )
}
