import { describe, expect, it } from 'vitest'
import type { Scene, SceneBlock, SceneFrame } from '@/lib/api'
import { framesByScene, FRAME, VIDEO } from '@/lib/scenes'
import { checkStoryboard, type ComplaintKind } from '@/lib/storyboard'

const BLOCKS: SceneBlock[] = [
  { key: 'still', label: 'Still' },
  { key: 'motion', label: 'Motion' },
]

/** A scene written out in full unless told otherwise. */
const scene = (position: number, over: Partial<Scene> = {}): Scene => ({
  id: `s${position}`,
  profile_id: 'p',
  work_id: 'w',
  position,
  section: null,
  starts_at: null,
  ends_at: null,
  shot_type: null,
  description: 'a room at dusk',
  blocks: { still: 'a room', motion: 'slow push in' },
  created_at: '',
  updated_at: '',
  ...over,
})

const material = (id: string, sceneId: string, kind: string, isSelected: boolean): SceneFrame => ({
  id,
  scene_id: sceneId,
  asset_id: `a-${id}`,
  kind,
  position: 1,
  is_selected: isSelected,
  path: `/media/${id}`,
  original_name: null,
  created_at: '',
})

/** A scene with its still and its clip both chosen: nothing to complain about. */
const shot = (position: number, over: Partial<Scene> = {}) => ({
  scene: scene(position, over),
  frames: [
    material(`f${position}`, `s${position}`, FRAME, true),
    material(`v${position}`, `s${position}`, VIDEO, true),
  ],
})

const kinds = (board: ReturnType<typeof checkStoryboard>): ComplaintKind[] =>
  board.complaints.map((complaint) => complaint.kind)

describe('what a board is still missing', () => {
  it('names the earliest thing in the way, one line per scene', () => {
    // Four scenes at four stages of the same work. The point is that each is
    // told the ONE thing to do next: scene 4 is not also told it has no clip.
    const untouched = scene(1, { description: '', blocks: {} })
    const half = scene(2, { blocks: { still: 'a room' } })
    const written = scene(3)
    const framed = scene(4)

    const board = checkStoryboard(
      [untouched, half, written, framed],
      BLOCKS,
      framesByScene([material('f4', 's4', FRAME, true)]),
      null,
    )

    expect(kinds(board)).toEqual(['empty', 'noPrompt', 'noFrame', 'noVideo'])
    expect(board.complaints.map((complaint) => complaint.scene?.position)).toEqual([1, 2, 3, 4])
  })

  it('tells a scene waiting on a verdict apart from one nobody has drawn', () => {
    // Three pictures back from the generator and none chosen is a different
    // job from a scene nobody has drawn at all: one needs a decision, the
    // other needs a generator, and a person reading the list is deciding
    // which of the two to go and do.
    const board = checkStoryboard(
      [scene(1), scene(2)],
      BLOCKS,
      framesByScene([
        material('a', 's1', FRAME, false),
        material('b', 's1', FRAME, false),
        material('c', 's1', FRAME, false),
      ]),
      null,
    )

    expect(kinds(board)).toEqual(['undecidedFrame', 'noFrame'])
    expect(board.complaints[0]?.count).toBe(3)
  })

  it('tells an undecided clip apart from a missing one, the same way', () => {
    const framed = (position: number) => material(`f${position}`, `s${position}`, FRAME, true)
    const board = checkStoryboard(
      [scene(1), scene(2)],
      BLOCKS,
      framesByScene([
        framed(1),
        material('v1', 's1', VIDEO, false),
        material('v2', 's1', VIDEO, false),
        framed(2),
      ]),
      null,
    )

    expect(kinds(board)).toEqual(['undecidedVideo', 'noVideo'])
    expect(board.complaints[0]?.count).toBe(2)
  })

  it('does not take a chosen clip for a chosen still', () => {
    // The v0.69 rule, held here too: a board where the clip was picked and
    // the picture never was is not a board that has been shot.
    const board = checkStoryboard(
      [scene(1)],
      BLOCKS,
      framesByScene([material('v1', 's1', VIDEO, true)]),
      null,
    )

    expect(kinds(board)).toEqual(['noFrame'])
    expect(board.tally.framed).toBe(0)
    expect(board.tally.filmed).toBe(1)
  })

  it('is silent about a board where nothing has been timed yet', () => {
    // A board is timed late and all at once. Fifty "no timing" lines would
    // be fifty lines about one thing that has not happened yet.
    const board = checkStoryboard(
      [shot(1).scene, shot(2).scene],
      BLOCKS,
      framesByScene([...shot(1).frames, ...shot(2).frames]),
      120,
    )

    expect(board.complaints).toEqual([])
    expect(board.done).toBe(true)
  })

  it('finds a hole between two scenes, and an overlap, and says how big', () => {
    const board = checkStoryboard(
      [
        shot(1, { starts_at: 0, ends_at: 10 }).scene,
        // Starts two seconds after the one before ended.
        shot(2, { starts_at: 12, ends_at: 20 }).scene,
        // Starts half a second before the one before ended.
        shot(3, { starts_at: 19.5, ends_at: 30 }).scene,
      ],
      BLOCKS,
      framesByScene([...shot(1).frames, ...shot(2).frames, ...shot(3).frames]),
      30,
    )

    expect(kinds(board)).toEqual(['gap', 'overlap'])
    expect(board.complaints[0]?.seconds).toBeCloseTo(2)
    expect(board.complaints[0]?.scene?.position).toBe(2)
    expect(board.complaints[1]?.seconds).toBeCloseTo(0.5)
    expect(board.complaints[1]?.scene?.position).toBe(3)
  })

  it('lets a rounding tail pass, because the timing it checks did the rounding', () => {
    // `scene::timings` rounds every edge to a tenth, so a person who then
    // types an edge by hand leaves hundredths behind: scene 2 begins a
    // hundredth after scene 1 ended, and the board ends a hundredth short of
    // the length. Holding these to exact equality would report the
    // division's own arithmetic as holes in the video.
    const board = checkStoryboard(
      [
        shot(1, { starts_at: 0, ends_at: 3.33 }).scene,
        shot(2, { starts_at: 3.34, ends_at: 6.67 }).scene,
        shot(3, { starts_at: 6.66, ends_at: 9.99 }).scene,
      ],
      BLOCKS,
      framesByScene([...shot(1).frames, ...shot(2).frames, ...shot(3).frames]),
      10,
    )

    expect(board.complaints).toEqual([])
  })

  it('counts the untimed scenes once, and does not call the hole they leave a gap', () => {
    // Scene 2 has no span. It is named once, as a count; the edges of 1 and 3
    // are then compared to each other rather than across it, because the
    // missing span is already the complaint.
    const board = checkStoryboard(
      [
        shot(1, { starts_at: 0, ends_at: 10 }).scene,
        shot(2).scene,
        shot(3, { starts_at: 10, ends_at: 20 }).scene,
      ],
      BLOCKS,
      framesByScene([...shot(1).frames, ...shot(2).frames, ...shot(3).frames]),
      20,
    )

    expect(kinds(board)).toEqual(['untimed'])
    expect(board.complaints[0]?.count).toBe(1)
  })

  it('says when the board does not cover the length of the work', () => {
    const board = checkStoryboard(
      [shot(1, { starts_at: 0, ends_at: 10 }).scene],
      BLOCKS,
      framesByScene(shot(1).frames),
      45,
    )

    expect(kinds(board)).toEqual(['length'])
    expect(board.complaints[0]?.seconds).toBeCloseTo(35)
  })

  it('says nothing about the length when the work has none', () => {
    const board = checkStoryboard(
      [shot(1, { starts_at: 0, ends_at: 10 }).scene],
      BLOCKS,
      framesByScene(shot(1).frames),
      null,
    )

    expect(board.complaints).toEqual([])
  })

  it('counts a board the way a person would count it by hand', () => {
    const board = checkStoryboard(
      [
        shot(1, { starts_at: 0, ends_at: 10 }).scene,
        scene(2), // written, nothing drawn
        scene(3, { description: '' }), // not written
      ],
      BLOCKS,
      framesByScene([
        ...shot(1).frames,
        // A still chosen for 3 although it is not written out: the tally
        // counts facts, and these two are separate facts.
        material('f3', 's3', FRAME, true),
      ]),
      null,
    )

    expect(board.tally).toEqual({ scenes: 3, written: 2, framed: 2, filmed: 1, timed: 1 })
  })

  it('does not call an empty board finished', () => {
    // Nothing missing because there is nothing there. A board with no scenes
    // has not been done, it has not been started.
    const board = checkStoryboard([], BLOCKS, framesByScene([]), 120)

    expect(board.complaints).toEqual([])
    expect(board.done).toBe(false)
    expect(board.tally.scenes).toBe(0)
  })

  it('asks only for a description when the kind names no prompt blocks', () => {
    // A kind with no blocks is ready as soon as it is described — the rule
    // `readinessOf` already holds, reused here rather than restated.
    const board = checkStoryboard([shot(1).scene], [], framesByScene(shot(1).frames), null)

    expect(board.complaints).toEqual([])
    expect(board.tally.written).toBe(1)
  })
})

describe('a run of scenes saying the same thing', () => {
  /** A board of `count` scenes, every one written out and nothing drawn. */
  const written = (count: number) =>
    Array.from({ length: count }, (_, index) => scene(index + 1))

  it('collapses the shape a real board actually makes', () => {
    // Measured on the owner's board, 2026-09-16: fifty scenes written out in
    // full, not one picture drawn, no timings. Fifty identical lines is one
    // fact typed fifty times, and it pushes everything else off the screen.
    const board = checkStoryboard(written(50), BLOCKS, framesByScene([]), 225)

    expect(board.complaints).toHaveLength(1)
    expect(board.complaints[0]).toMatchObject({
      kind: 'noFrame',
      count: 50,
      run: { from: 1, to: 50 },
    })
    // The counts still say the board is written: the panel does not lose the
    // good news along with the repetition.
    expect(board.tally).toMatchObject({ scenes: 50, written: 50, framed: 0 })
  })

  it('sends the person to the FIRST of a run, where the work resumes', () => {
    const board = checkStoryboard(written(5), BLOCKS, framesByScene([]), null)

    expect(board.complaints[0]?.run?.sceneId).toBe('s1')
  })

  it('leaves two alone, because two lines are two scenes', () => {
    const board = checkStoryboard(written(2), BLOCKS, framesByScene([]), null)

    expect(board.complaints).toHaveLength(2)
    expect(board.complaints.every((complaint) => complaint.run === undefined)).toBe(true)
  })

  it('does not join scenes that are not neighbours', () => {
    // Three holes in work that is otherwise done are three separate jobs, and
    // showing them separately is the whole point of the list. Scenes 1, 3
    // and 5 want a picture; 2 and 4 have one and want a clip.
    const drawn = (position: number) => material(`f${position}`, `s${position}`, FRAME, true)
    const board = checkStoryboard(written(5), BLOCKS, framesByScene([drawn(2), drawn(4)]), null)

    expect(kinds(board)).toEqual(['noFrame', 'noVideo', 'noFrame', 'noVideo', 'noFrame'])
    expect(board.complaints.every((complaint) => complaint.run === undefined)).toBe(true)
  })

  it('does not join three of a KIND that are scattered across the board', () => {
    // The sharper form of the same rule: six complaints, all `noFrame`, but
    // they fall on scenes 1, 3, 5, 7, 9 and 11 with drawn-and-filmed scenes
    // between them. Nothing may collapse — three scattered gaps are three
    // places to go, and a single line saying "scenes 1 to 11" would send the
    // person to a stretch that is mostly finished.
    const done = (position: number) => [
      material(`f${position}`, `s${position}`, FRAME, true),
      material(`v${position}`, `s${position}`, VIDEO, true),
    ]
    const board = checkStoryboard(
      written(11),
      BLOCKS,
      framesByScene([2, 4, 6, 8, 10].flatMap(done)),
      null,
    )

    expect(kinds(board)).toEqual(Array.from({ length: 6 }, () => 'noFrame'))
    expect(board.complaints.every((complaint) => complaint.run === undefined)).toBe(true)
    expect(board.complaints.map((complaint) => complaint.scene?.position)).toEqual([
      1, 3, 5, 7, 9, 11,
    ])
  })

  it('collapses a run and leaves the odd one out standing beside it', () => {
    // Scenes 1 to 4 want a picture; scene 5 has four and wants a verdict.
    // One line for the stretch, one for the scene that is genuinely at a
    // different stage.
    const board = checkStoryboard(
      written(5),
      BLOCKS,
      framesByScene([
        material('a', 's5', FRAME, false),
        material('b', 's5', FRAME, false),
      ]),
      null,
    )

    expect(board.complaints).toHaveLength(2)
    expect(board.complaints[0]).toMatchObject({ kind: 'noFrame', count: 4, run: { from: 1, to: 4 } })
    expect(board.complaints[1]).toMatchObject({ kind: 'undecidedFrame', count: 2 })
    expect(board.complaints[1]?.run).toBeUndefined()
  })

  it('does not sum what the scenes hold, only how many they are', () => {
    // Three scenes with four candidates each is "three scenes waiting on a
    // verdict", not "twelve pictures" — a number answering a question nobody
    // asked.
    const candidates = [1, 2, 3].flatMap((position) =>
      [1, 2, 3, 4].map((index) => material(`f${position}-${index}`, `s${position}`, FRAME, false)),
    )
    const board = checkStoryboard(written(3), BLOCKS, framesByScene(candidates), null)

    expect(board.complaints).toHaveLength(1)
    expect(board.complaints[0]?.count).toBe(3)
  })

  it('never hides a board-wide complaint inside a run', () => {
    // The timing complaints have no scene, so they cannot be swept into a
    // stretch of scenes that happens to end beside them.
    const board = checkStoryboard(
      [
        shot(1, { starts_at: 0, ends_at: 10 }).scene,
        shot(2, { starts_at: 10, ends_at: 20 }).scene,
        shot(3, { starts_at: 20, ends_at: 30 }).scene,
      ],
      BLOCKS,
      framesByScene([
        ...shot(1).frames,
        ...shot(2).frames,
        ...shot(3).frames,
      ]),
      60,
    )

    expect(kinds(board)).toEqual(['length'])
    expect(board.complaints[0]?.run).toBeUndefined()
  })
})

describe('a board carrying every kind of complaint at once', () => {
  /*
   * The shape seeded into a sandbox copy to look at the panel with eyes
   * (2026-09-16): fifty scenes, the first ten timed with a hole and an
   * overlap in them, three finished, one waiting on a verdict for its
   * picture, one for its clip, one untouched, one half written, and the
   * long tail undrawn. It is here as a test because what the panel prints
   * on a board like that is the whole question this stage answers, and a
   * screenshot cannot be re-run next year.
   */
  it('says each thing once, in the board order, with the tail as one line', () => {
    const drawn = (position: number, over: Partial<Scene> = {}) => scene(position, over)
    const scenes: Scene[] = [
      drawn(1, { starts_at: 0, ends_at: 10 }),
      drawn(2, { starts_at: 10, ends_at: 20 }),
      drawn(3, { starts_at: 20, ends_at: 30 }),
      drawn(4, { starts_at: 30, ends_at: 40 }),
      drawn(5, { starts_at: 40, ends_at: 50 }),
      drawn(6, { starts_at: 50, ends_at: 60, description: '', blocks: {} }),
      // Three seconds of nothing before it, and one prompt block missing.
      drawn(7, { starts_at: 63, ends_at: 70, blocks: { still: 'a room' } }),
      drawn(8, { starts_at: 70, ends_at: 80 }),
      // Two seconds of overlap with the one before.
      drawn(9, { starts_at: 78, ends_at: 90 }),
      drawn(10, { starts_at: 90, ends_at: 100 }),
      ...Array.from({ length: 40 }, (_, index) => drawn(index + 11)),
    ]

    const held = [
      // 1-3 finished.
      ...[1, 2, 3].flatMap((position) => [
        material(`f${position}`, `s${position}`, FRAME, true),
        material(`v${position}`, `s${position}`, VIDEO, true),
      ]),
      // 4: four pictures, no verdict.
      ...[1, 2, 3, 4].map((index) => material(`f4-${index}`, 's4', FRAME, false)),
      // 5: picture chosen, two clips, no verdict.
      material('f5', 's5', FRAME, true),
      ...[1, 2].map((index) => material(`v5-${index}`, 's5', VIDEO, false)),
    ]

    const board = checkStoryboard(scenes, BLOCKS, framesByScene(held), 225)

    expect(kinds(board)).toEqual([
      'undecidedFrame', // 4
      'undecidedVideo', // 5
      'empty', // 6
      'noPrompt', // 7
      'noFrame', // 8 to 50, as one line
      'untimed',
      'gap',
      'overlap',
      'length',
    ])

    // The tail is one line about forty-three scenes, not forty-three lines.
    const tail = board.complaints[4]
    expect(tail?.run).toEqual({ from: 8, to: 50, sceneId: 's8' })
    expect(tail?.count).toBe(43)

    // Nine lines for a fifty-scene board in this state: a list, not a wall.
    expect(board.complaints).toHaveLength(9)

    expect(board.tally).toEqual({ scenes: 50, written: 48, framed: 4, filmed: 3, timed: 10 })
  })
})
