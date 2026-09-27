import { QueryClient } from '@tanstack/react-query'

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
