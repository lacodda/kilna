/**
 * A plugin's word in the window's language.
 *
 * A manifest carries one `label` and, optionally, the same word by language
 * (`labels: { ru: '...' }`). The window takes its own language's, by the base
 * of its tag, and falls back to the plugin's own word: a plugin that says
 * nothing about Russian still has a button in a Russian window.
 */
export function inLanguage(
  language: string,
  word: string,
  words: Readonly<Record<string, string | undefined>>,
): string {
  const base = language.split('-')[0] ?? language
  return words[language] ?? words[base] ?? word
}
