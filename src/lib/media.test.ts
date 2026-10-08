import { describe, expect, it } from 'vitest'
import { extensionOf, mediaKindOf, nameOf } from '@/lib/media'

describe('mediaKindOf', () => {
  it('tells a picture, a clip and anything else apart by the ending', () => {
    expect(mediaKindOf('C:\\ws\\media\\a1.png')).toBe('picture')
    expect(mediaKindOf('/ws/media/a1.JPEG')).toBe('picture')
    expect(mediaKindOf('C:\\ws\\media\\take.mp4')).toBe('clip')
    expect(mediaKindOf('/ws/media/take.MOV')).toBe('clip')
    expect(mediaKindOf('/ws/media/notes.pdf')).toBe('other')
    expect(mediaKindOf('/ws/media/noending')).toBe('other')
  })

  it('knows a sound by the formats the player opens (v0.93)', () => {
    for (const name of ['mix.wav', 'mix.MP3', 'stem.flac', 'take.m4a', 'voice.ogg', 'a.opus']) {
      expect(mediaKindOf(`D:/songs/${name}`), name).toBe('sound')
    }
  })

  it('names a file by the last step of its path, on every system', () => {
    expect(nameOf('C:\\media\\songs\\mix.wav')).toBe('mix.wav')
    expect(nameOf('/media/songs/mix.wav')).toBe('mix.wav')
  })

  it('reads the ending of the file, not of a folder on the way to it', () => {
    expect(extensionOf('C:\\my.folder\\file')).toBe('')
    expect(extensionOf('/home/user/.hidden')).toBe('')
    expect(extensionOf('/a/b/c.tar.gz')).toBe('gz')
  })
})
