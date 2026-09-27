# 37. Reads, writes and run events go through one layer

Date: 2026-09-27

## Status

Accepted.

## Context

The audit of 24.09 counted how the frontend talked to its backend. `lib/api.ts`
was 1,850 lines: 139 types copied from the Rust structs by hand and 170
wrappers, in the order they were written. Ninety-two `useQuery` calls wrote
their key and their function side by side, thirteen of them with a key made up
on the spot - one list was read under two spellings of the same key. The 122
`useMutation` calls each wrote an `onError` (sixty-one the same line) and a
list of what to refresh (twenty-nine lists, a screen added later in some of
them). Seven components listened to the assistant's run events, each keeping
its own corner of the cache.

## Decision

**The wire is split by domain.** `lib/api/types.ts` holds the contract with the
backend and nothing else; it is the file the backend will generate when the
types come from Rust. `lib/api/<domain>.ts` holds the wrappers of one domain -
works, versions, releases, the assistant - and an import names the module the
thing lives in. `src/test/ipc.test.ts` holds the two sides to one list of
commands.

**A read is a factory.** `queries.work(id)` is the key and the function
together; a screen adds only what is its own (`enabled`, `select`, a refetch
interval). Every key lives in `lib/query/keys.ts`.

**A write is `useAppMutation`.** It names what it disturbed by kind -
`refresh.note`, `refresh.version(workId)` - from `lib/query/refresh.ts`, always
refreshes the journal (every write a person makes is in the log, ADR 0033),
and says a failure in the words of the gesture (`failure: 'toast.noteSaveFailed'`)
or the backend's. A write that must put something back first (`onMutate`, a
rollback) or settle whatever happened (`onSettled`) stays a plain `useMutation`.

**Run events are heard once.** `RunEventsBridge`, mounted with the providers,
listens to `assistant:run` and `assistant:queue`, lands every event on its run
in the cache and refreshes the lists a run's start or end moves. A component
that must act when a run ends - announce it, show its answer - asks
`useRunEvent`. The drawer and a card's Assistant tab share one list of chats
(`features/assistant/chats.tsx`) and differ only in how they draw it.

## Consequences

- A new screen reads through a factory and writes through the hook, so what it
  refreshes and how it fails are decided by the kind of thing it touches.
- A key can no longer be spelled two ways, and a list of chats or running
  tasks is current on every screen, whichever screen started the run.
- The types still agree with Rust only by hand until they are generated; the
  split makes that a one-file change.
