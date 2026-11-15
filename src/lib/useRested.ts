import { useDebounced } from '@/lib/useDebounced'

/**
 * How long a pointer, or a highlight, stays on one thing before it is asked
 * about. A chip carried to Friday crosses Monday to Thursday on the way, and
 * a dry run ordered for each of them is four answers nobody reads - which the
 * backend works through one at a time, with the date written on letting go
 * standing in line behind them (measured in v0.96: 4-6 s). A pause this long
 * is not felt as one; a crossing is shorter.
 */
const REST_MS = 150

/**
 * Whether `value` has stayed what it is for `REST_MS`: false from the moment
 * it changes until it has rested.
 */
export function useRested<T>(value: T): boolean {
  return useDebounced(value, REST_MS) === value
}
