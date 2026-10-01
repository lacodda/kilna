import { invoke } from '@tauri-apps/api/core'
import type {
  NewScene,
  Scene,
  SceneFrame,
  SceneFrameView,
  SceneNote,
  ScenePatch,
} from '@/lib/api/types'

// A work's storyboard: scenes, their notes and their frames.

export const listScenes = (workId: string) => invoke<Scene[]>('list_scenes', { workId })
export const createScene = (scene: NewScene) => invoke<Scene>('create_scene', { scene })
export const updateScene = (id: string, patch: ScenePatch) =>
  invoke<Scene>('update_scene', { id, patch })
export const deleteScene = (id: string) => invoke<string>('delete_scene', { id })
/** Number a board in the order given: 1..N, in one change. The whole order
    travels because both gestures — putting a scene between two others and
    moving one that is there — are the same thing said twice. */
export const renumberScenes = (workId: string, ids: string[]) =>
  invoke<Scene[]>('renumber_scenes', { workId, ids })
/** Divide the work's length between the scenes of its board. */
export const timeScenes = (workId: string) => invoke<Scene[]>('time_scenes', { workId })
/** Build the board's frame from the parts the source text marks out. */
export const frameScenes = (workId: string, role: string) =>
  invoke<Scene[]>('frame_scenes', { workId, role })
export const listSceneNotes = (workId: string) =>
  invoke<SceneNote[]>('list_scene_notes', { workId })
export const attachSceneNote = (sceneId: string, noteId: string) =>
  invoke<SceneNote>('attach_scene_note', { sceneId, noteId })
export const detachSceneNote = (sceneId: string, noteId: string) =>
  invoke<void>('detach_scene_note', { sceneId, noteId })
export const listSceneFrames = (workId: string) =>
  invoke<SceneFrame[]>('list_scene_frames', { workId })
export const attachSceneFrame = (sceneId: string, kind: string, source: string) =>
  invoke<SceneFrame>('attach_scene_frame', { sceneId, kind, source })
/** A pasted picture: the clipboard gives bytes, so the bytes are what travels. */
export const pasteSceneFrame = (sceneId: string, kind: string, bytes: Uint8Array, name: string) =>
  invoke<SceneFrame>('paste_scene_frame', { sceneId, kind, bytes: Array.from(bytes), name })
export const detachSceneFrame = (id: string) => invoke<void>('detach_scene_frame', { id })
/** A scene's built frame: its still written around the picture block in the
 *  clip's style, and the scheme of where its hero stands. */
export const sceneFrameView = (id: string) => invoke<SceneFrameView>('scene_frame_view', { id })
export const selectSceneFrame = (id: string) => invoke<SceneFrame>('select_scene_frame', { id })
/** Put one kind of a scene's material in the order given, first to last: the
 * whole order travels, as it does for the board's scenes. */
export const reorderSceneFrames = (sceneId: string, kind: string, ids: string[]) =>
  invoke<SceneFrame[]>('reorder_scene_frames', { sceneId, kind, ids })
/** Take back the verdict on one kind: unchoosing a clip says nothing about
 * the still it was animated from. */
export const clearSceneFrame = (sceneId: string, kind: string) =>
  invoke<void>('clear_scene_frame', { sceneId, kind })
