/**
 * Reading the address without being on a route.
 *
 * The topbar is a sibling of `<Routes>` rather than a descendant, so `useParams`
 * there matches nothing and always reads empty. It parses the path itself.
 */

/** The work in `/works/:workId[/:tab]`, or `undefined` on any other address. */
export function openWorkId(pathname: string): string | undefined {
  const [, screen, id] = pathname.split('/')
  return screen === 'works' && id !== undefined && id !== '' ? id : undefined
}
