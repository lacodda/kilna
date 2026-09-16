import type { Scene, SceneBlock, SceneFrame } from '@/lib/api'
import { FRAME, VIDEO, chosenFrame, chosenVideo, ofKind, readinessOf } from '@/lib/scenes'

/*
 * What a board is still missing, and how much of it is done.
 *
 * A storyboard is finished long before it looks finished. Fifty rows all
 * carrying a number and a description read as a full board at a glance, and
 * the four without a prompt, the gap between scene 11 and 12, and the minute
 * of video that is not accounted for are found later — by the person, in the
 * editing program, with the files already in hand. This is the pass that
 * finds them first.
 *
 * Nothing here is stored. Like the derived status and the release's
 * readiness, every count is computed from what the board holds right now, so
 * a check can never describe a board as it was before the last keystroke.
 *
 * It computes on the screen's side (decision of 2026-09-16) and reads the
 * same facts the rows are drawn from, using the very same `readinessOf`: the
 * report saying a scene is shot while the row says otherwise would be two
 * truths about one scene, and reusing the function is what makes that
 * impossible rather than merely unlikely.
 */

/** What is wrong with one scene, or with the board's timing. */
export type ComplaintKind =
  /** Described, but some prompt block the kind asks for is empty. */
  | 'noPrompt'
  /** Prompts written and not one picture drawn: this needs a generator. */
  | 'noFrame'
  /** Pictures drawn and none chosen: this needs a decision. */
  | 'undecidedFrame'
  /** A still chosen and no clip at all. */
  | 'noVideo'
  /** Clips there and none chosen. */
  | 'undecidedVideo'
  /** Neither a description nor a prompt: a row nobody has touched. */
  | 'empty'
  /** This scene starts after the one before it ended. */
  | 'gap'
  /** This scene starts before the one before it ended. */
  | 'overlap'
  /** Some scenes are timed and some are not. */
  | 'untimed'
  /** The spans do not add up to the work's length. */
  | 'length'

/** One thing found, about one scene or about the board. */
export interface Complaint {
  kind: ComplaintKind
  /** The scene it is about; absent for the ones that are about the board,
   * and for a run collapsed into one line. */
  scene?: Scene
  /** Seconds — how big the gap, the overlap, or the difference is. */
  seconds?: number
  /** How many scenes or pictures, for a complaint that counts. */
  count?: number
  /** A run of neighbouring scenes saying the same thing, collapsed into one
   * line: the first and the last of them, and where to go. */
  run?: { from: number; to: number; sceneId: string }
}

/**
 * How many neighbours saying the same thing become one line.
 *
 * Two identical lines read as two scenes; ten read as a wall, and a wall
 * pushes the board's real problems off the screen. Measured on a real board
 * (2026-09-16): fifty scenes written out in full and not one picture drawn
 * produced fifty identical lines, which is one fact typed fifty times rather
 * than a queue of fifty jobs. Three is where a list stops being a list.
 */
const A_RUN = 3

/** How much of the board is done, in the numbers a person counts by hand. */
export interface Tally {
  scenes: number
  /** Described and every prompt block written: ready to draw. */
  written: number
  /** A still chosen. */
  framed: number
  /** A clip chosen. */
  filmed: number
  /** Carrying a span at both ends. */
  timed: number
}

/** What a board says about itself: the numbers, and what is missing. */
export interface Storyboard {
  tally: Tally
  complaints: Complaint[]
  /** Nothing is missing: every scene is written, framed and filmed, and the
   * timing holds together. */
  done: boolean
}

/**
 * How close two seconds have to be to count as the same instant.
 *
 * A board is timed by dividing a length, and the division rounds to a tenth
 * (`scene::timings`); a person then drags an edge and types `1:23.5`. Holding
 * those to exact equality would report a rounding tail as a gap in the video,
 * which is the check crying wolf about arithmetic it did itself. A twentieth
 * of a second is below what any of this is measuring and above every rounding
 * this code performs.
 */
const SAME_INSTANT = 0.05

/** Whether a scene carries a span at both ends. */
function isTimed(scene: Scene): boolean {
  return scene.starts_at !== null && scene.ends_at !== null
}

/**
 * Everything one board has to say about itself.
 *
 * `blocks` is the kind's prompt blocks, `frames` the board's material by scene
 * id, `duration` the work's length in seconds when it has one.
 *
 * The complaints come in the board's own order — scene by scene, then the ones
 * that speak about the whole timing — because a list that jumps around is a
 * list a person has to sort before they can work through it.
 */
export function checkStoryboard(
  scenes: Scene[],
  blocks: SceneBlock[],
  frames: Map<string, SceneFrame[]>,
  duration: number | null,
): Storyboard {
  const complaints: Complaint[] = []
  const tally: Tally = { scenes: scenes.length, written: 0, framed: 0, filmed: 0, timed: 0 }

  for (const scene of scenes) {
    const material = frames.get(scene.id)
    const readiness = readinessOf(scene, blocks, material ?? [])
    const framed = chosenFrame(material) !== undefined
    const filmed = chosenVideo(material) !== undefined

    if (readiness !== 'empty' && readiness !== 'started') tally.written += 1
    if (framed) tally.framed += 1
    if (filmed) tally.filmed += 1
    if (isTimed(scene)) tally.timed += 1

    // One complaint per scene, the earliest thing standing in the way. A row
    // with no description has no business being told its clip is missing too:
    // the list is a queue of what to do next, and four lines about one
    // untouched scene push the board's real problems off the screen.
    if (readiness === 'empty') complaints.push({ kind: 'empty', scene })
    else if (readiness === 'started') complaints.push({ kind: 'noPrompt', scene })
    else if (!framed) {
      // Four pictures back from the generator with no verdict is a different
      // job from a scene nobody has drawn: one needs a decision, the other
      // needs a generator. Two complaints rather than one with a count,
      // because they send the person to two different places.
      const drawn = ofKind(material, FRAME).length
      complaints.push(
        drawn === 0
          ? { kind: 'noFrame', scene }
          : { kind: 'undecidedFrame', scene, count: drawn },
      )
    } else if (!filmed) {
      const cut = ofKind(material, VIDEO).length
      complaints.push(
        cut === 0 ? { kind: 'noVideo', scene } : { kind: 'undecidedVideo', scene, count: cut },
      )
    }
  }

  complaints.push(...timingComplaints(scenes, duration))

  return {
    tally,
    complaints: collapse(complaints),
    done: scenes.length > 0 && complaints.length === 0,
  }
}

/**
 * Runs of neighbouring scenes saying the same thing, as one line each.
 *
 * A board moves through its stages in one piece: every scene is written
 * before any is drawn, and every one is drawn before any is cut. So the
 * complaints a real board produces are not fifty different jobs — they are
 * one job, and the honest way to say that is once, with how many and from
 * where to where.
 *
 * Only NEIGHBOURS collapse, and only the same kind. Scenes 3 to 42 waiting
 * for a picture is one line; scenes 3, 9 and 40 waiting while the ones
 * between them are drawn is three, because that is three separate gaps in
 * work that is otherwise done, and the whole point of the list is to show
 * them. The count carried by a complaint — four pictures waiting on a
 * verdict — is deliberately not summed: a run is about the scenes, not
 * about what they hold, and "138 pictures" would answer a question nobody
 * asked.
 */
function collapse(complaints: Complaint[]): Complaint[] {
  const out: Complaint[] = []
  let index = 0
  while (index < complaints.length) {
    const first = complaints[index]
    if (first === undefined) break
    let end = index
    // Neighbours on the board, not merely neighbours in this list: a scene
    // whose own complaint was a different kind breaks the run, and so it
    // should — the work is not uniform across it.
    while (end + 1 < complaints.length) {
      const next = complaints[end + 1]
      const held = complaints[end]
      if (
        next === undefined ||
        held === undefined ||
        next.kind !== first.kind ||
        next.scene === undefined ||
        held.scene === undefined ||
        next.scene.position !== held.scene.position + 1
      ) {
        break
      }
      end += 1
    }

    const length = end - index + 1
    const last = complaints[end]
    if (length >= A_RUN && first.scene !== undefined && last?.scene !== undefined) {
      out.push({
        kind: first.kind,
        count: length,
        run: {
          from: first.scene.position,
          to: last.scene.position,
          // The first of them is where the person is sent: it is where the
          // work resumes.
          sceneId: first.scene.id,
        },
      })
    } else {
      out.push(...complaints.slice(index, end + 1))
    }
    index = end + 1
  }
  return out
}

/**
 * What the spans say about each other and about the length.
 *
 * A board is timed late and all at once, so a board where *nothing* is timed
 * is not a board with fifty problems — it is a board at an earlier stage of
 * the work, and saying nothing about timing is the honest answer. Only a
 * board that is partly timed has scenes that are missing theirs.
 */
function timingComplaints(scenes: Scene[], duration: number | null): Complaint[] {
  const timed = scenes.filter(isTimed)
  if (timed.length === 0) return []

  const complaints: Complaint[] = []

  // Some timed and some not: the untimed ones are named as a count rather
  // than one line each, because they are one unfinished gesture, not a list
  // of independent oversights.
  if (timed.length < scenes.length) {
    complaints.push({ kind: 'untimed', count: scenes.length - timed.length })
  }

  // Edges are compared in the board's order, between neighbours that are both
  // timed: an untimed scene in the middle is already reported above, and
  // treating it as a hole would report the same fact twice.
  for (let index = 1; index < timed.length; index += 1) {
    const before = timed[index - 1]
    const scene = timed[index]
    if (before === undefined || scene === undefined) continue
    const difference = (scene.starts_at ?? 0) - (before.ends_at ?? 0)
    if (Math.abs(difference) <= SAME_INSTANT) continue
    complaints.push({
      kind: difference > 0 ? 'gap' : 'overlap',
      scene,
      seconds: Math.abs(difference),
    })
  }

  // The sum against the length: the board covers the video, or it does not.
  // Measured as the last end against the length rather than by adding the
  // spans up, because a gap or an overlap is already its own complaint and
  // adding them in again would report one mistake as two.
  if (duration !== null && duration > 0) {
    const covered = timed[timed.length - 1]?.ends_at ?? 0
    const difference = covered - duration
    if (Math.abs(difference) > SAME_INSTANT) {
      complaints.push({ kind: 'length', seconds: Math.abs(difference) })
    }
  }

  return complaints
}
