import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router'
import { QueryClientProvider } from '@tanstack/react-query'
import App from '@/App'
import { ErrorBoundary } from '@/components/ErrorBoundary'
import { Toaster } from '@/components/Toaster'
import { initTheme } from '@/lib/theme'
import { initLanguage } from '@/lib/language'
import { queryClient } from '@/lib/query'
import '@/i18n'
import '@/styles.css'

// Apply the stored theme and language before the first paint, so the window
// never flashes the wrong colour or visibly changes language a moment in.
initTheme()
initLanguage()

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      {/* Wraps the app rather than sitting beside it: a toast raised from
          anywhere inside has to reach the same provider. */}
      <Toaster>
        <BrowserRouter>
          {/* Outermost boundary: catches a crash in the shell itself, which the
              per-screen boundary inside App cannot reach. */}
          <ErrorBoundary>
            <App />
          </ErrorBoundary>
        </BrowserRouter>
      </Toaster>
    </QueryClientProvider>
  </React.StrictMode>,
)
