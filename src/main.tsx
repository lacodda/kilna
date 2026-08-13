import React from 'react'
import ReactDOM from 'react-dom/client'
import App from '@/App'
import { initTheme } from '@/lib/theme'
import '@/i18n'
import '@/styles.css'

// Apply the stored theme before the first paint so the window never flashes.
initTheme()

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
