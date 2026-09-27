// The layout of the code, settled by a tool rather than by whoever typed last.
//
// The options are the style the codebase already had by hand - no semicolons,
// single quotes, trailing commas - so the one commit that applied it moved
// whitespace, not habits. The width is the one real choice: 100 holds the
// longest lines this code writes on purpose (a query key beside its function,
// a class list on a JSX element) without folding them, and still fits two
// files side by side.
/** @type {import('prettier').Config} */
export default {
  semi: false,
  singleQuote: true,
  trailingComma: 'all',
  printWidth: 100,
}
