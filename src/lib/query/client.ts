import { focusManager, QueryClient } from '@tanstack/react-query'

/**
 * A client with the app's defaults. A factory rather than only the one
 * instance, so each test starts from an empty cache instead of inheriting the
 * answers of the test before it.
 */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        // Local SQLite behind an IPC call: refetching is cheap, but not free
        // enough to do on every window focus while someone is typing.
        staleTime: 30_000,
        refetchOnWindowFocus: false,
        // A missing row will not become present by asking again, and a broken
        // query should surface now rather than after three silent retries.
        retry: false,
      },
      mutations: { retry: false },
    },
  })
}

/** The window's one client. */
export const queryClient = createQueryClient()

// What counts as the window coming back: its focus, not only its visibility.
// The library listens for `visibilitychange`, which a desktop window rarely
// sends - switching to another program leaves it visible - so a query that
// asks to be looked at again on focus (a work's folder on disk, written into
// by other programs) would wait for a minimise to notice anything.
focusManager.setEventListener((handleFocus) => {
  const focused = () => handleFocus(true)
  const blurred = () => handleFocus(false)
  window.addEventListener('focus', focused)
  window.addEventListener('blur', blurred)
  return () => {
    window.removeEventListener('focus', focused)
    window.removeEventListener('blur', blurred)
  }
})
