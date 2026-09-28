import type { Tab } from '@/features/work/tabs'
import { IDS } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The places a test can open the window at, on the studio of
 * `test/workspace.ts`: every screen, and every tab each kind of work draws.
 *
 * Each names a line of the studio the place must show. Without one, a screen
 * stuck on its skeleton - or on the splash - would pass every other check.
 */

export const SCREENS: [path: string, shows: string][] = [
  ['/dashboard', 'Harbour Lights'],
  ['/catalogue', 'Harbour Lights'],
  ['/calendar', 'Paper Lanterns (clip)'],
  ['/notes', 'A song about tides'],
  [`/notes/${IDS.note}`, 'Something about the tide going out and taking the day with it.'],
  ['/comments', 'The shot on the bridge is beautiful.'],
  [`/comments/${IDS.comment}`, 'Paper Lanterns (clip)'],
  ['/styles', 'Dusk over water'],
  [`/styles/${IDS.brick}`, en.styles.describe],
  ['/journal', '“Harbour Lights” added.'],
  ['/trash', 'An old idea'],
  ['/settings', en.nav.data],
]

/** The tabs a kind draws, each with a line of the studio it must show. */
export const CARD_TABS: [workId: string, tab: Tab, shows: string][] = [
  [IDS.song, 'overview', 'BPM'],
  [IDS.song, 'versions', 'Second pass'],
  [IDS.song, 'score', 'Does the chorus stay with you after one listen?'],
  [IDS.song, 'releases', 'Sep 22'],
  [IDS.song, 'files', 'Paper Lanterns'],
  [IDS.song, 'links', 'Paper Lanterns (clip)'],
  [IDS.song, 'notes', 'Paper Lanterns'],
  [IDS.song, 'comments', 'Paper Lanterns'],
  [IDS.song, 'assistant', 'Tighten the chorus'],
  [IDS.song, 'history', '“Paper Lanterns” scored 7.5.'],
  [IDS.video, 'overview', 'Paper Lanterns (clip)'],
  [IDS.video, 'versions', 'A girl lets a paper lantern go at dusk'],
  [IDS.video, 'scenes', 'The lantern drifts under the bridge.'],
  [IDS.video, 'cuts', 'Paper Lanterns'],
  [IDS.video, 'score', 'Paper Lanterns (clip)'],
  [IDS.video, 'releases', 'Paper Lanterns (clip)'],
  [IDS.video, 'files', 'Paper Lanterns (clip)'],
  [IDS.video, 'links', 'soundtrack'],
  [IDS.video, 'notes', 'An old man who sells lanterns by the bridge.'],
  [IDS.video, 'comments', 'The shot on the bridge is beautiful.'],
  [IDS.video, 'assistant', 'Paper Lanterns (clip)'],
  [IDS.video, 'history', 'Paper Lanterns (clip)'],
  [IDS.short, 'cuts', 'Paper Lanterns (clip)'],
]
