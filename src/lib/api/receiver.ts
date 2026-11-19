import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  ArmedReceiver,
  MediaRole,
  ReceiverEnded,
  ReceiverFound,
  ReceiverSettings,
  TakenMedia,
} from '@/lib/api/types'

// The armed receiver (v0.156, FR-21 21.2): one work waits for the file a
// generator is about to save in Downloads.

export const receiverSettings = () => invoke<ReceiverSettings>('receiver_settings')

export const setReceiverSettings = (settings: ReceiverSettings) =>
  invoke<ReceiverSettings>('set_receiver_settings', { settings })

export const armedReceiver = () => invoke<ArmedReceiver | null>('armed_receiver')

export const armReceiver = (workId: string, role: MediaRole) =>
  invoke<ArmedReceiver>('arm_receiver', { workId, role })

export const disarmReceiver = () => invoke<null>('disarm_receiver')

export const takeReceived = (workId: string, role: MediaRole, path: string) =>
  invoke<TakenMedia>('take_received', { workId, role, path })

/** Hear what arrives for the receiver. */
export const onReceiverFound = (handler: (found: ReceiverFound) => void): Promise<UnlistenFn> =>
  listen<ReceiverFound>('receiver-found', (event) => handler(event.payload))

/** Hear a receiver stop by itself, its time run out. */
export const onReceiverEnded = (handler: (ended: ReceiverEnded) => void): Promise<UnlistenFn> =>
  listen<ReceiverEnded>('receiver-ended', (event) => handler(event.payload))
