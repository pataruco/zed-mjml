# Talk: From HTML emails to Rust

A short talk, about 13 minutes, on why this project exists and what it is built from: Zed, tree-sitter, a language server, and WebAssembly with WASI. The WebAssembly section draws on Pedro's post [WASM: beyond the browser and back again](https://pataruco.dev/blog/wasm-beyond-the-browser-and-back-again/).

The deck is written in [Typst](https://typst.app) with the [touying](https://touying-typ.github.io) slides package, and styled after [Awesome Badger](https://awesome.red-badger.com), the Red Badger blog. The spoken script lives in the speaker notes inside `main.typ`, so the slides and the words have one source of truth.

## Build

Install Typst first. On macOS: `brew install typst`. The packages the deck uses download automatically on the first compile.

```sh
# Slides only
typst compile --font-path talk/fonts talk/main.typ talk/slides.pdf

# Slides with the speaker notes beside each slide, for rehearsing or a second screen
typst compile --font-path talk/fonts --input notes=true talk/main.typ talk/notes.pdf

# Live preview while editing
typst watch --font-path talk/fonts talk/main.typ talk/slides.pdf
```

`--font-path talk/fonts` loads the deck's fonts. To skip typing it, set `TYPST_FONT_PATHS=talk/fonts` in your shell. Compiled PDFs are ignored by git.

## Present

- Put `slides.pdf` on the projector and `notes.pdf` on your laptop, or use any PDF presenter that shows a notes file on the second screen.
- Running short on time? Three slides are marked "Short version" at the top of their notes: the two validation passes, the spectrum of WebAssembly uses, and the release pipeline. Skip them and the talk runs about 9 minutes.
- Fill in the placeholders in `main.typ` before the day. They are marked `TODO`: the event and date on the title slide, your first-job detail in the opening notes, the number of years on the closing slide, and the Zed screenshot, which is a dashed box until you drop an image in.

## Files

- `main.typ`: the slides and the speaker notes
- `snippets/`: the code excerpts shown on the slides. They are short snapshots taken from this repo and from MJML's compiled output, trimmed on purpose so they read on a projector.
- `assets/`: the Awesome Badger and Red Badger logos, from the blog's repository
- `fonts/`: the typefaces the deck uses, with their licences

## Style

The look follows the blog's stylesheet: the `#F8F8F8` page, `#111` text, the `#FC1D42` brand red for emphasis, grey `#969696` captions, the dark rule across the top of every slide, and the small Red Badger logo in the footer.

The blog sets text in Proxima Nova and headlines in Sentinel. Both are licensed to the website only, so they are not in this repo. The deck ships two open stand-ins under the SIL Open Font License instead: [Figtree](https://github.com/erikdkennedy/figtree) for Proxima Nova and [Zilla Slab](https://github.com/mozilla/zilla-slab) for Sentinel. If you have the brand fonts installed on your machine, put them first in the font lists at the top of `main.typ` and Typst will use them.

## How the talk is shaped

The talk follows content design principles: give people what they need, when they need it, in the form they expect.

- Front-loaded: the one-line description of the project arrives within 30 seconds, before any concept is explained.
- One concept per slide, in the order the audience needs them: editor, then parser, then server, then WebAssembly, then the sandbox it runs in. Each builds on the one before.
- The WebAssembly section moves from the general to the specific: what Wasm is, three ways Pedro has used it, why WASI exists, and a side-by-side of a browser extension and this Zed extension. The contrast is what makes WASI click.
- Plain words first, project example second: every term is defined in one sentence, then shown with this repo's own code.
- Diagrams where prose fails: the editor-to-server exchange and the way the parts fit together are drawn, not described.
- Specific over vague: real line counts, dates and platform names replace adjectives like "small" or "recent".
- Written to be spoken: the notes use short sentences and active voice, one idea per sentence.
- Slides carry one picture each and no bullet lists. The notes carry the words.

Alternatives if something does not land:

- Title: "Bookends: an email tool, written in Rust" or "From Outlook tables to wasm32-wasip2".
- Opening: start with the problem instead of the biography. Show a broken `.mjml` file with no feedback, then the same file with a red underline, then the bookends.
- The WebAssembly analogy: "a kitchen with no taps, and the standard plumbing" if the sockets line feels flat.
