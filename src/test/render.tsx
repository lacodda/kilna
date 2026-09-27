import { MemoryRouter } from 'react-router'
import { render, renderHook, waitFor } from '@testing-library/react'
import type { QueryClient } from '@tanstack/react-query'
import App from '@/app/App'
import { Providers } from '@/app/Providers'
import { ErrorBoundary } from '@/components/ErrorBoundary'
import { createQueryClient } from '@/lib/query'

/**
 * The whole window at `path`: the same providers and boundary as `main.tsx`,
 * a router that starts where the test says, and a query cache of its own.
 */
export function renderApp(path: string) {
  const client = createQueryClient()
  const view = render(
    <Providers client={client}>
      <MemoryRouter initialEntries={[path]}>
        <ErrorBoundary>
          <App />
        </ErrorBoundary>
      </MemoryRouter>
    </Providers>,
  )
  return { ...view, client }
}

/** Until nothing is being fetched or written: the screen has said what it will say. */
export async function settled(client: QueryClient): Promise<void> {
  await waitFor(() => {
    if (client.isFetching() + client.isMutating() > 0) throw new Error('still loading')
  })
}

/** A hook inside the app's providers: the query cache, the language, toasts. */
export function renderHookWithin<Result, Props>(
  hook: (props: Props) => Result,
  initialProps?: Props,
) {
  const client = createQueryClient()
  const view = renderHook(hook, {
    initialProps,
    wrapper: ({ children }) => <Providers client={client}>{children}</Providers>,
  })
  return { ...view, client }
}
