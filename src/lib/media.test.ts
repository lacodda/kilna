import { describe, expect, it } from 'vitest'
import { extensionOf, mediaKindOf } from '@/lib/media'

describe('mediaKindOf', () => {
  it('tells a picture, a clip and anything else apart by the ending', () => {
    expect(mediaKindOf('C:\\ws\\media\\a1.png')).toBe('picture')
    expect(mediaKindOf('/ws/media/a1.JPEG')).toBe('picture')
    expect(mediaKindOf('C:\\ws\\media\\take.mp4')).toBe('clip')
    expect(mediaKindOf('/ws/media/take.MOV')).toBe('clip')
    expect(mediaKindOf('/ws/media/song.mp3')).toBe('other')
    expect(mediaKindOf('/ws/media/noending')).toBe('other')
  })

  it('reads the ending of the file, not of a folder on the way to it', () => {
    expect(extensionOf('C:\\my.folder\\file')).toBe('')
    expect(extensionOf('/home/user/.hidden')).toBe('')
    expect(extensionOf('/a/b/c.tar.gz')).toBe('gz')
  })
})
