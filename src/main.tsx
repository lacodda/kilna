import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router'
import { QueryClientProvider } from '@tanstack/react-query'
import { getCurrentWindow } from '@tauri-apps/api/window'
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

// The window is created hidden (tauri.conf.json) and shown from here, once
// the document has painted its splash in the right colours. Shown before it
// had, the webview flashed white for as long as the bundle took to arrive -
// which was the whole of the owner's first impression every morning. Outside
// Tauri (vitest, a browser tab) there is no window to show, and that is fine.
getCurrentWindow()
  .show()
  .catch(() => undefined)

/*
 * The webview's own right-click menu does not belong in a desktop app.
 *
 * "Back", "Reload", "Save as", "Print", "Inspect" are a browser's answers, and
 * this is not a browser: none of them mean anything over a song, and the first
 * two can lose what is being typed. They also cover the app's own context
 * menus, which is what the owner photographed.
 *
 * Not everywhere, though. Inside a text field the menu is the only way to Cut,
 * Copy and Paste with the mouse, and there is no app menu to replace it - so a
 * field, a textarea and anything `contenteditable` keep theirs, as does a
 * selection someone has just made in order to copy it. Everywhere else the
 * gesture belongs to the app, and a component that wants it takes it with its
 * own handler.
 */
document.addEventListener('contextmenu', (event) => {
  const target = event.target
  if (!(target instanceof Element)) return
  if (target.closest('input, textarea, [contenteditable=""], [contenteditable="true"]')) return
  if ((window.getSelection()?.toString() ?? '') !== '') return
  event.preventDefault()
})

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
