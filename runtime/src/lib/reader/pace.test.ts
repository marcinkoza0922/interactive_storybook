import { describe, expect, it } from 'vitest'
import type { Bundle, Page } from '../bundle/types'
import {
  DEFAULT_WPM,
  PageClock,
  emptyPace,
  minutesLeft,
  shortTimeLeft,
  timeLeftLabel,
  withPage,
  wordsPerMinute,
} from './pace'

const page = (words?: number): Page => ({ blocks: [{ id: 'b', html: '' }], words })

const bundle: Bundle = {
  bundle_schema_version: 1,
  book: { id: 'test', title: 'Test', author: 'A', language: 'en' },
  chapters: [
    { id: 'one', title: 'One', content_hash: 'h1', pages: [page(230), page(460), page(690)] },
    { id: 'old', title: 'Old', content_hash: 'h2', pages: [page()] },
  ],
}

/** A page of `words` read at `wpm`. */
const read = (words: number, wpm: number) => [words, (words / wpm) * 60_000] as const

describe('pace', () => {
  it('starts from the default', () => {
    expect(wordsPerMinute(emptyPace())).toBeCloseTo(DEFAULT_WPM)
  })

  it('moves toward the measured pace', () => {
    let pace = emptyPace()
    for (let i = 0; i < 3; i++) pace = withPage(pace, ...read(200, 400))
    const wpm = wordsPerMinute(pace)
    expect(wpm).toBeGreaterThan(300)
    expect(wpm).toBeLessThan(400)
    for (let i = 0; i < 50; i++) pace = withPage(pace, ...read(200, 400))
    expect(wordsPerMinute(pace)).toBeCloseTo(400, -1)
  })

  it('ignores skipped pages and pages left open', () => {
    const pace = emptyPace()
    expect(withPage(pace, ...read(200, 3000))).toEqual(pace)
    expect(withPage(pace, ...read(200, 20))).toEqual(pace)
    expect(withPage(pace, 0, 5000)).toEqual(pace)
  })

  it('forgets older reading', () => {
    let pace = emptyPace()
    for (let i = 0; i < 100; i++) pace = withPage(pace, ...read(200, 150))
    expect(pace.words).toBeLessThanOrEqual(5000)
    for (let i = 0; i < 100; i++) pace = withPage(pace, ...read(200, 400))
    expect(wordsPerMinute(pace)).toBeGreaterThan(380)
  })
})

describe('minutesLeft', () => {
  it('counts from the start of the current page to the chapter end', () => {
    expect(minutesLeft(bundle, { chapter: 0, page: 0 }, emptyPace())).toBeCloseTo(6)
    expect(minutesLeft(bundle, { chapter: 0, page: 2 }, emptyPace())).toBeCloseTo(3)
  })

  it('has no estimate without word counts', () => {
    expect(minutesLeft(bundle, { chapter: 1, page: 0 }, emptyPace())).toBeNull()
  })
})

describe('timeLeftLabel', () => {
  it('rounds to whole minutes', () => {
    expect(timeLeftLabel(8.6)).toBe('About 9 minutes left in this chapter')
    expect(timeLeftLabel(1.2)).toBe('About 1 minute left in this chapter')
    expect(timeLeftLabel(0.4)).toBe('Less than a minute left in this chapter')
    expect([shortTimeLeft(8.6), shortTimeLeft(0.4)]).toEqual(['9 min', '<1 min'])
  })
})

describe('PageClock', () => {
  it('leaves out paused time', () => {
    let now = 0
    const clock = new PageClock(() => now)
    clock.restart()
    now = 1000
    clock.setPaused(true)
    now = 5000
    expect(clock.elapsed()).toBe(1000)
    clock.setPaused(false)
    now = 6000
    expect(clock.elapsed()).toBe(2000)
  })

  it('restarts while paused without counting the pause', () => {
    let now = 0
    const clock = new PageClock(() => now)
    clock.setPaused(true)
    now = 3000
    clock.restart()
    now = 4000
    clock.setPaused(false)
    now = 4500
    expect(clock.elapsed()).toBe(500)
  })
})
