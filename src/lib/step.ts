/**
 * A number held to its setting's step and bounds: what a box that goes by
 * ones keeps when 18.5 is typed into it is 19, not 18.5 (wish 6815). The
 * steps count from `min` when there is one, from nought otherwise - the way
 * the backend checks them (`plugin::manifest::on_step`).
 */
export function toStep(value: number, step?: number, min?: number, max?: number): number {
  let held = value
  if (step !== undefined && Number.isFinite(step) && step > 0) {
    const base = min ?? 0
    // Rounded to the step, then cut to the decimals the step itself has, so
    // 0.1 + 0.2 does not come back as 0.30000000000000004.
    const decimals = (String(step).split('.')[1] ?? '').length
    held = Number((base + Math.round((held - base) / step) * step).toFixed(decimals))
  }
  if (min !== undefined) held = Math.max(min, held)
  if (max !== undefined) held = Math.min(max, held)
  return held
}
