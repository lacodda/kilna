# Russian language pack

What kilna knows about Russian words without asking anyone (ADR 0053):
where a word form may be stressed, which of its vowels are written ё, and how
common its stem is. Both files are finite-state maps (the `fst` crate),
compiled into the binary and read where they lie.

| File | What it maps | Built from | Licence |
| --- | --- | --- | --- |
| `stress.fst` | a word form, lowercase with ё read as е → its stresses, its ё, whether the е spelling is a word too | the dictionaries of [ruaccent](https://huggingface.co/ruaccent/accentuator/tree/main/dictionary): `accents`, `omographs`, `yo_words`, `yo_homographs` | MIT, © Denis Petrov ([ruaccent](https://github.com/Den4ikAI/ruaccent)) |
| `frequency.fst` | a stem, as kilna's stemmer writes it → its rank by frequency, 1 the commonest (the first 100 000) | Koziev's [word-form frequencies](https://github.com/Koziev/NLP_Datasets/tree/master/WordformFrequencies) (`term2freq`) | CC0 1.0 |

The ruaccent licence, whose notice travels with every copy of `stress.fst`,
is in [`LICENSE-ruaccent`](LICENSE-ruaccent).

## Rebuilding

Download the sources and unpack them into one directory: the four
`.json.gz` files of ruaccent's `dictionary/` folder as `accents.json`,
`omographs.json`, `yo_words.json`, `yo_homographs.json`, and `term2freq.7z`
as `term2freq.dat`. Then, from `src-tauri/`:

```
cargo run --release --example lang_pack -- <sources> lang/ru
```

The frequency list is keyed by the stemmer (`src/words/stem.rs`): rebuild it
whenever the stemmer changes, or rarity is read off keys nothing produces any
more. `words::pack`'s tests hold the pack to a few known answers.
