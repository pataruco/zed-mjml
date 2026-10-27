# Talk: From HTML emails to Rust

A short talk, about 10 minutes, on why this project exists and what it is built from: Zed, tree-sitter, a language server, and WebAssembly with WASI.

The deck is written in [Typst](https://typst.app) with the [touying](https://touying-typ.github.io) slides package. The spoken script lives in the speaker notes inside `main.typ`, so the slides and the words have one source of truth.

## Build

Install Typst first. On macOS: `brew install typst`. The packages the deck uses download automatically on the first compile.

```sh
# Slides only
typst compile talk/main.typ talk/slides.pdf

# Slides with the speaker notes beside each slide, for rehearsing or a second screen
typst compile --input notes=true talk/main.typ talk/notes.pdf

# Live preview while editing
typst watch talk/main.typ talk/slides.pdf
```

Compiled PDFs are ignored by git.

## Present

- Put `slides.pdf` on the projector and `notes.pdf` on your laptop, or use any PDF presenter that shows a notes file on the second screen.
- Running short on time? Two slides are marked "5-minute cut" at the top of their notes: the two validation passes, and the release pipeline. Skip both and the talk fits in 5 minutes.
- Fill in the placeholders in `main.typ` before the day. They are marked `TODO`: the event and date on the title slide, your first-job detail in the opening notes, the number of years on the closing slide, and the Zed screenshot, which is a dashed box until you drop an image in.

## Files

- `main.typ`: the slides and the speaker notes
- `snippets/`: the code excerpts shown on the slides. They are short snapshots taken from this repo and from MJML's compiled output, trimmed on purpose so they read on a projector.

## How the talk is shaped

The talk follows content design principles: give people what they need, when they need it, in the form they expect.

- Front-loaded: the one-line description of the project arrives within 30 seconds, before any concept is explained.
- One concept per slide, in the order the audience needs them: editor, then parser, then server, then sandbox. Each builds on the one before.
- Plain words first, project example second: every term is defined in one sentence, then shown with this repo's own code.
- Diagrams where prose fails: the editor-to-server exchange and the way the parts fit together are drawn, not described.
- Specific over vague: real line counts, dates and platform names replace adjectives like "small" or "recent".
- Written to be spoken: the notes use short sentences and active voice, one idea per sentence.
- Slides carry one picture each and no bullet lists. The notes carry the words.

Alternatives if something does not land:

- Title: "Bookends: an email tool, written in Rust" or "From Outlook tables to wasm32-wasip2".
- Opening: start with the problem instead of the biography. Show a broken `.mjml` file with no feedback, then the same file with a red underline, then the bookends.
- The WebAssembly analogy: "a kitchen with no taps, and the standard plumbing" if the sockets line feels flat.
