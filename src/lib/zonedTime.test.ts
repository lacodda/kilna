import { describe, expect, it } from 'vitest'
import { inZone, zoneLabel, zoneNames } from './zonedTime'

describe('inZone', () => {
  it('reads an hour said in one zone on the clock of another', () => {
    // Asuncion is UTC-3 all year since 2024; Moscow is UTC+3.
    expect(
      inZone({ date: '2026-10-14', time: '11:00' }, 'America/Asuncion', 'Europe/Moscow'),
    ).toEqual({ date: '2026-10-14', time: '17:00' })
    expect(inZone({ date: '2026-10-14', time: '11:00' }, 'America/Asuncion', 'UTC')).toEqual({
      date: '2026-10-14',
      time: '14:00',
    })
  })

  it('moves the day when the hour crosses midnight, either way', () => {
    expect(
      inZone({ date: '2026-10-14', time: '23:30' }, 'Europe/Moscow', 'America/Asuncion'),
    ).toEqual({ date: '2026-10-14', time: '17:30' })
    expect(
      inZone({ date: '2026-10-14', time: '01:30' }, 'Europe/Moscow', 'America/Asuncion'),
    ).toEqual({ date: '2026-10-13', time: '19:30' })
    expect(
      inZone({ date: '2026-12-31', time: '22:00' }, 'America/Asuncion', 'Europe/Moscow'),
    ).toEqual({ date: '2027-01-01', time: '04:00' })
  })

  it('follows a zone across its own change of clocks', () => {
    // Berlin is UTC+2 in summer and UTC+1 in winter; the same 12:00 is two
    // different hours in UTC.
    expect(inZone({ date: '2026-07-01', time: '12:00' }, 'Europe/Berlin', 'UTC').time).toBe('10:00')
    expect(inZone({ date: '2026-12-01', time: '12:00' }, 'Europe/Berlin', 'UTC').time).toBe('11:00')
  })

  it('is the same hour in the same zone', () => {
    const at = { date: '2026-03-29', time: '02:30' }
    expect(inZone(at, 'UTC', 'UTC')).toEqual(at)
  })
})

describe('zoneNames', () => {
  it('holds UTC and the names a release is said in', () => {
    const names = zoneNames()
    expect(names).toContain('UTC')
    expect(names).toContain('America/Asuncion')
    expect(names).toContain('Europe/Moscow')
  })
})

describe('zoneLabel', () => {
  it('turns the underscores of a name back into spaces', () => {
    expect(zoneLabel('America/Argentina/Buenos_Aires')).toBe('America/Argentina/Buenos Aires')
  })
})
