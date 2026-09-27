import '@testing-library/jest-dom/vitest'
import { afterEach } from 'vitest'
import { cleanup } from '@testing-library/react'
import { clearMocks } from '@tauri-apps/api/mocks'
import i18n from '@/i18n'

/*
 * What jsdom does not have and the window does. Each is the smallest stand-in
 * that lets a component mount; none of them pretends to lay anything out, so
 * a test that needs a measured size has to say what the size is.
 */
class Unobserved {
  observe() {}
  unobserve() {}
  disconnect() {}
  takeRecords() {
    return []
  }
}
globalThis.ResizeObserver ??= Unobserved as unknown as typeof ResizeObserver
globalThis.IntersectionObserver ??= Unobserved as unknown as typeof IntersectionObserver

window.matchMedia ??= (query: string) =>
  ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
  }) as MediaQueryList

Element.prototype.scrollIntoView ??= function scrollIntoView() {}
Element.prototype.scrollTo ??= function scrollTo() {}
Element.prototype.hasPointerCapture ??= () => false
Element.prototype.releasePointerCapture ??= () => {}
Element.prototype.getAnimations ??= () => []

afterEach(async () => {
  cleanup()
  // An event listener is taken down when the promise that registered it
  // settles, a turn after the unmount; clearing the mocks first leaves it
  // nothing to unregister from.
  await new Promise((resolve) => setTimeout(resolve, 0))
  clearMocks()
  localStorage.clear()
  await i18n.changeLanguage('en')
})
