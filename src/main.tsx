import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router'
import { QueryClientProvider } from '@tanstack/react-query'
import App from '@/App'
import { ErrorBoundary } from '@/components/ErrorBoundary'
import { Toaster } from '@/components/Toaster'
import { initTheme } from '@/lib/theme'
import { queryClient } from '@/lib/query'
import '@/i18n'
import '@/styles.css'

// Apply the stored theme before the first paint so the window never flashes.
initTheme()

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        {/* Outermost boundary: catches a crash in the shell itself, which the
            per-screen boundary inside App cannot reach. */}
        <ErrorBoundary>
          <App />
        </ErrorBoundary>
      </BrowserRouter>
      <Toaster />
    </QueryClientProvider>
  </React.StrictMode>,
)
