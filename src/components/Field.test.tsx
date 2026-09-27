import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import { Field } from './Field'

// Rendered to markup rather than mounted: what matters is the shape the
// browser receives, and the shape alone decides where a click on the caption
// goes.

describe('Field', () => {
  it('never wraps a row of buttons in a label', () => {
    const html = renderToStaticMarkup(
      <Field label="Type" hint="What the description answers">
        <div>
          <button type="button">Image style</button>
          <button type="button">Character</button>
        </div>
      </Field>,
    )
    // A label around buttons forwards any click on the caption, the hint or
    // a gap to the first of them - the defect this shape exists to prevent.
    expect(html).not.toContain('<label')
    expect(html).toMatch(/^<div role="group" aria-labelledby="([^"]+)"/)
    const group = /aria-labelledby="([^"]+)"/.exec(html)?.[1]
    expect(html).toContain(`<span id="${group}"`)
  })

  it('treats a block of content as a group, not as a control', () => {
    const html = renderToStaticMarkup(
      <Field label="Prompt">
        <pre>text</pre>
      </Field>,
    )
    expect(html).not.toContain('<label')
    expect(html).toContain('role="group"')
  })

  it('ties its caption to a single control by id, and the hint by description', () => {
    const html = renderToStaticMarkup(
      <Field label="Name" hint="Shown in the list">
        <input />
      </Field>,
    )
    const labelFor = /<label [^>]*for="([^"]+)"/.exec(html)?.[1]
    const inputId = /<input [^>]*id="([^"]+)"/.exec(html)?.[1]
    expect(labelFor).toBeDefined()
    expect(labelFor).toBe(inputId)
    // The label holds the caption and nothing else.
    expect(html).toMatch(/<label [^>]*>Name<\/label>/)
    const described = /aria-describedby="([^"]+)"/.exec(html)?.[1]
    expect(html).toContain(`<span id="${described}"`)
  })

  it('keeps an id the control already has', () => {
    const html = renderToStaticMarkup(
      <Field label="Name">
        <input id="work-title" />
      </Field>,
    )
    expect(html).toContain('for="work-title"')
    expect(html).toContain('id="work-title"')
  })
})
