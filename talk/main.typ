// Talk: From HTML emails to Rust. How I built MJML for Zed.
//
// Build the slides:            typst compile talk/main.typ talk/slides.pdf
// Build slides + notes:        typst compile --input notes=true talk/main.typ talk/notes.pdf
//
// The spoken script lives in the #speaker-note blocks under each slide.
// Things to fill in before the day are marked TODO.

#import "@preview/touying:0.8.0": *
#import themes.simple: *
#import "@preview/fletcher:0.5.8" as fletcher: diagram, node, edge

#let with-notes = sys.inputs.at("notes", default: "false") == "true"

// Colours
#let zed-blue = rgb("#084CCF") // the blue of the Zed badge in the README
#let sandboxed = rgb("#FFF1D6") // parts that run inside Zed's WebAssembly sandbox
#let native = rgb("#E2F3E4") // the one part that runs as a native process
#let muted = luma(110)

#show: simple-theme.with(
  aspect-ratio: "16-9",
  primary: zed-blue,
  footer: [MJML for Zed],
  config-common(show-notes-on-second-screen: if with-notes { right } else { none }),
  config-info(
    title: [From HTML emails to Rust],
    subtitle: [How I built MJML for Zed],
    author: [Pedro Martin],
    date: datetime.today(),
  ),
)

#set text(font: ("Helvetica Neue", "Libertinus Serif"))
#show raw: set text(font: ("Menlo", "DejaVu Sans Mono"))
#show raw.where(block: true): it => block(
  fill: luma(247),
  stroke: luma(225) + 0.5pt,
  inset: 10pt,
  radius: 6pt,
  width: 100%,
  it,
)

// A small grey caption above a snippet or picture.
#let caption(it) = text(size: 0.65em, fill: muted, it)

// A code excerpt from snippets/, at a size that reads on a projector.
#let snippet(file, lang: none, size: 12pt) = text(
  size: size,
  raw(read("snippets/" + file), lang: lang, block: true),
)

// A dashed box where a screenshot goes.
// Replace the call with: image("assets/zed-error.png", width: 100%)
#let placeholder(what, height: 7.5cm) = rect(
  width: 100%,
  height: height,
  radius: 8pt,
  fill: luma(251),
  stroke: (paint: luma(170), dash: "dashed", thickness: 1.5pt),
  align(center + horizon, text(size: 0.7em, fill: muted, what)),
)

// A red marker at the top of the notes of a slide you can skip.
#let cut = text(fill: red, weight: "bold")[5-minute cut: skip this slide if short on time.]

// ---------------------------------------------------------------------------

#title-slide[
  #text(1.5em, weight: "bold", fill: zed-blue)[From HTML emails to Rust]
  #v(0.2em)
  #text(1.1em)[How I built MJML for Zed]
  #v(1.2em)
  #text(0.7em, fill: muted)[Pedro Martin · TODO event and date]
]

// ---------------------------------------------------------------------------

== My first job was HTML emails. My current one is Rust.

#grid(
  columns: (1fr, 1fr),
  column-gutter: 1em,
  [#caption[Then: an email template, one of thousands] #snippet("then.html", lang: "html")],
  [#caption[Now: `src/lib.rs` in this repo] #snippet("now.rs", lang: "rust")],
)

#speaker-note[
  My first job in tech was building HTML emails. Nested tables, inline styles, and testing everything in Outlook. _TODO: swap in your real first-job detail._

  Today I write Rust.

  This project joins those two ends. It started as an experiment. It is now a published Zed extension called MJML for Zed. It gives you syntax highlighting, live error checking, completions and docs for MJML, the email markup language.

  In the next ten minutes I'll explain the four things it is built from: Zed, tree-sitter, a language server, and WebAssembly. Then I'll show you how they fit together on one slide.
]

// ---------------------------------------------------------------------------

== MJML: you write nine lines, email clients get a wall of tables

#grid(
  columns: (1fr, 1.4fr),
  column-gutter: 1em,
  [#caption[You write] #snippet("hello.mjml", lang: "html")],
  [
    #caption[Email clients get]
    #block(height: 5.8cm, clip: true, radius: 6pt, snippet("compiled.html", lang: "html", size: 8.5pt))
    #caption[... and it keeps going.]
  ],
)

#speaker-note[
  Email clients don't render modern HTML. They want tables and inline styles, like it's 2005.

  MJML fixes that. It is an open-source markup language made by Mailjet. You write sections, columns and text. It compiles to the nested tables and conditional comments that email clients tolerate.

  Notice one thing: MJML looks exactly like HTML with custom tag names. The whole extension relies on that.
]

// ---------------------------------------------------------------------------

== Zed: a code editor written in Rust

#placeholder[TODO screenshot: Zed with a `.mjml` file open and one error underlined in red]
#v(0.4em)
#align(center, caption[Written in Rust · draws its interface on the GPU · extensions are Rust compiled to WebAssembly])

#speaker-note[
  Zed is a code editor. It is written in Rust by the people who built Atom and tree-sitter. It is fast because it draws its interface on the GPU, the way a game does.

  Two things matter for this talk. Zed extensions are written in Rust and compiled to WebAssembly, not JavaScript. And Zed is the editor I use every day. It didn't understand `.mjml` files, so I had an itch to scratch.
]

// ---------------------------------------------------------------------------

== Tree-sitter turns your file into a tree, and keeps it up to date as you type

#grid(
  columns: (1fr, 1fr),
  column-gutter: 1em,
  row-gutter: 0.6em,
  [#caption[Your file] #snippet("tree-input.mjml", lang: "html")],
  grid.cell(rowspan: 2)[#caption[The tree, simplified] #snippet("tree.txt", size: 11pt)],
  [#caption[A query from `languages/mjml/highlights.scm`] #snippet("highlights.scm", lang: "scheme", size: 11pt)],
)

#speaker-note[
  Tree-sitter is a parser. You give it a grammar for a language, and it builds a syntax tree of your file. Every tag, attribute and comment becomes a node.

  Two things make it special. It is incremental: when you type one character, it updates only the part of the tree that changed. And it tolerates errors: a half-typed tag doesn't break highlighting for the rest of the file.

  The editor doesn't read the tree directly. It asks questions of it with query files, which end in `.scm`. A query says: "find every tag name that matches mjml, mj-head or mj-body, and colour it as a keyword."

  Here is the trick I mentioned. Because MJML is HTML with custom tag names, I didn't write a grammar. I reused tree-sitter-html, pinned to one commit, and wrote six query files: highlighting, bracket matching, indentation, outline, and CSS highlighting inside `mj-style`. 113 lines in total.
]

// ---------------------------------------------------------------------------

== A language server knows what the file means

#align(center, diagram(
  node-stroke: 1.2pt,
  node-corner-radius: 6pt,
  node-inset: 10pt,
  node-shape: rect,
  spacing: (9em, 1em),
  node((0, 0), [*Editor* \ #text(0.7em)[Zed]], name: <ed>, fill: luma(245)),
  node((1, 0), [*Language server* \ #text(0.7em)[`mjml-lsp`]], name: <ls>, fill: native),
  edge(<ed>, <ls>, "->", text(0.55em, align(center)[document opened or changed \ what goes here? what is this? how do I fix it?]), bend: 24deg),
  edge(<ls>, <ed>, "->", text(0.55em, align(center)[errors and warnings \ completions, hover docs, quick fixes]), bend: 24deg),
))
#v(0.1em)
#align(center, caption[A separate process. JSON messages over standard input and output, both ways.])

#speaker-note[
  Highlighting is about how the file looks. Knowing that `mj-text` can't sit directly inside `mj-section` is about what the file means. That is a language server's job.

  The Language Server Protocol, LSP, is a standard Microsoft created for VS Code in 2016. Before it, every editor needed its own plugin for every language. With LSP you write one server, and every editor can use it.

  The mechanics are simple. The editor starts the server as a child process. They exchange JSON messages over standard input and output. The editor says "this document opened" or "this document changed". The server replies with diagnostics: "error at line 12, mj-text can't go here". The editor also asks questions: what can go here, what does this tag mean, how do I fix this. Those are completion, hover and code actions.
]

// ---------------------------------------------------------------------------

== mjml-lsp checks a document in two passes

#let pass(title, body) = block(
  fill: luma(247),
  stroke: luma(225) + 0.5pt,
  inset: 16pt,
  radius: 8pt,
  width: 100%,
  height: 5.8cm,
)[
  #text(0.9em, weight: "bold", fill: zed-blue, title)
  #v(0.5em)
  #text(0.62em, body)
]

#grid(
  columns: (1fr, 1fr),
  column-gutter: 1.2em,
  pass[1. Tag scanner + rules table][
    Nesting: `mj-text` inside `mj-section` \
    Required attributes: `src` on `mj-image` \
    Unknown tags, with "did you mean?" \
    A second `mj-head` or `mj-body`
  ],
  pass[2. mrml, a Rust port of MJML][
    Unclosed tags and malformed markup \
    Follows `mj-include`, so a template split into partials validates as a whole
  ],
)
#v(0.5em)
#align(center, caption[About 4,400 lines of Rust with tests. One table in `rules.rs` feeds validation, completion and hover.])

#speaker-note[
  #cut

  mjml-lsp is the Rust binary in this repo, about 4,400 lines including tests. It checks a document in two passes.

  First, my own scanner walks the tags and applies the MJML rules: nesting, required attributes, unknown tags with "did you mean", duplicate heads and bodies. The rules live in one table. Validation, completion and hover all read from it, so adding a component is one entry.

  Second, mrml, a Rust port of MJML, parses the document and catches structural problems: unclosed tags, malformed markup. It also follows `mj-include`, so a template split into partials validates as a whole.
]

// ---------------------------------------------------------------------------

== WebAssembly is a sealed box. WASI is the standard set of doors.

#let door(name, fill: luma(232)) = box(
  fill: fill,
  inset: (x: 8pt, y: 5pt),
  radius: 4pt,
  stroke: luma(150) + 0.8pt,
  text(0.5em, name),
)
#let sealed(title, doors) = [
  #align(center, caption(title))
  #v(0.3em)
  #rect(
    width: 100%,
    height: 5.5cm,
    stroke: luma(50) + 4pt,
    radius: 10pt,
    fill: luma(250),
    inset: 12pt,
  )[
    #v(0.3cm)
    #align(center, rect(fill: white, stroke: zed-blue + 1.5pt, radius: 6pt, inset: 14pt)[
      your code \ #text(0.6em, fill: muted)[compiled to Wasm]
    ])
    #place(bottom + center, dy: 2pt, doors)
  ]
]

#grid(
  columns: (1fr, 1fr),
  column-gutter: 1.2em,
  sealed[Wasm alone: no doors][],
  sealed[Wasm + WASI, plus the doors Zed adds][
    #stack(
      dir: ttb,
      spacing: 5pt,
      stack(dir: ltr, spacing: 5pt, door[files], door[clock], door[random], door[stdio], door[env]),
      stack(
        dir: ltr,
        spacing: 5pt,
        door(fill: zed-blue.lighten(78%))[GitHub release],
        door(fill: zed-blue.lighten(78%))[download],
        door(fill: zed-blue.lighten(78%))[run command],
      ),
    )
  ],
)
#v(0.3em)
#align(center, caption[Grey doors come from WASI. Blue doors are what Zed adds through its extension interface.])

#speaker-note[
  This is the part that confused me the most, so here is the version I wish someone had told me.

  WebAssembly, or Wasm, is a portable binary format. You compile Rust, C or Go to it, and it runs at near-native speed on any platform, inside a sandbox. The sandbox is the whole point. A Wasm module can't open a file, make a network call, or even read the clock. It can only call functions the host chooses to hand it.

  That is safe, but a program that can't touch anything isn't useful. So WASI, the WebAssembly System Interface, is a standard set of those functions: files, environment variables, clocks, random numbers, standard input and output. The host grants each one as a capability. Zed gives my extension its own folder and nothing else.

  Short version: Wasm is the engine. WASI is the standard sockets you plug it into.
]

// ---------------------------------------------------------------------------

== The glue compiles to `wasm32-wasip2` and exports one function

#grid(
  columns: (1fr, 1fr),
  column-gutter: 1em,
  [#caption[The contract, from Zed's extension API (WIT, excerpt)] #snippet("extension.wit", size: 10pt)],
  [#caption[`src/lib.rs`, 112 lines in total (condensed)] #snippet("glue.rs", lang: "rust", size: 10pt)],
)

#speaker-note[
  The extension here compiles to the `wasm32-wasip2` target. The "p2" is WASI preview 2, which adds the component model. The contract between Zed and my code is written in WIT files, a typed interface language. Zed imports functions into my module, like "latest GitHub release" and "download file". My module exports one function: "give me the language server command".

  That is why the glue is tiny: 112 lines. It works out the user's OS and CPU, finds the matching file on the latest GitHub release, downloads it once per version, marks it executable, and tells Zed to run it with `--stdio`.
]

// ---------------------------------------------------------------------------

== Three parts, two sandboxes, one native process

#let cell(body) = text(0.55em, body)
#align(center, diagram(
  node-stroke: 1pt,
  node-corner-radius: 6pt,
  node-inset: 9pt,
  node-shape: rect,
  spacing: (2em, 0.9em),
  node((0, -1), caption[*In the repo*], stroke: none),
  node((1, -1), caption[*Built by*], stroke: none),
  node((2, -1), caption[*Running inside Zed*], stroke: none),

  node((0, 0), cell[`extension.toml` \ `languages/mjml/*.scm`], name: <g0>),
  node((1, 0), cell[Zed fetches tree-sitter-html \ and compiles it to Wasm], name: <g1>),
  node((2, 0), cell[*Grammar* (Wasm, sandboxed) \ parses the file; the queries colour, \ indent, outline and inject CSS], name: <g2>, fill: sandboxed),
  edge(<g0>, <g1>, "->"),
  edge(<g1>, <g2>, "->"),

  node((0, 1), cell[`src/lib.rs`], name: <e0>),
  node((1, 1), cell[Zed compiles it to \ `wasm32-wasip2`], name: <e1>),
  node((2, 1), cell[*Extension* (Wasm + WASI, sandboxed) \ finds and downloads the server, \ returns the command to run], name: <e2>, fill: sandboxed),
  edge(<e0>, <e1>, "->"),
  edge(<e1>, <e2>, "->"),

  node((0, 2), cell[`crates/mjml-lsp`], name: <l0>),
  node((1, 2), cell[GitHub Actions builds 3 native \ binaries, attached to the release], name: <l1>),
  node((2, 2), cell[*`mjml-lsp --stdio`* (native process) \ diagnostics, completions, \ hover, quick fixes], name: <l2>, fill: native),
  edge(<l0>, <l1>, "->"),
  edge(<l1>, <l2>, "->"),

  edge(<e2>, <l2>, "->", text(0.5em)[Zed spawns it], label-side: right),
))

#speaker-note[
  Here is the whole thing on one slide.

  Three parts live in the repo: the query files, the extension glue, and the language server.

  Two of them run inside Zed's WebAssembly sandbox: the grammar and the glue. Zed compiles both to Wasm when the extension is built. One runs as a native process: the language server. That is how Zed runs every language server, rust-analyzer included.

  When you open a `.mjml` file: the grammar parses it and the queries colour it, instantly. The extension checks the latest GitHub release, downloads the binary for your platform if it doesn't have it yet, and hands Zed the command. Zed starts the server. From then on, every edit goes to the server and errors come back.
]

// ---------------------------------------------------------------------------

== Releases are automated. Commit messages decide the version.

#v(0.3em)
#align(center, diagram(
  node-stroke: 1pt,
  node-corner-radius: 6pt,
  node-inset: 9pt,
  node-shape: rect,
  spacing: (1em, 0.5em),
  node((0, 0), cell[Conventional commits land on `main`]),
  edge("->"),
  node((0, 1), cell[release-please keeps a release pull request open]),
  edge("->"),
  node((0, 2), cell[Merging it creates the tag `zed-mjml-v0.3.1` and a GitHub release], fill: luma(245)),
  edge("->"),
  node((0, 3), cell[The Deploy workflow builds 3 binaries and attaches them to the release], fill: native),
  edge("->"),
  node((0, 4), cell[A pull request to `zed-industries/extensions` publishes the new version]),
))
#v(0.5em)
#align(center, caption[macOS Apple Silicon · macOS Intel · Linux x86-64])

#speaker-note[
  #cut

  Releases are automated too. Conventional commits drive release-please, which keeps a release pull request open. Merging it creates the tag. A workflow builds the server for macOS on Apple Silicon and Intel, and for Linux, and attaches the three binaries to the release. A pull request to Zed's extension registry publishes the new version.
]

// ---------------------------------------------------------------------------

#focus-slide[
  Same problem, TODO N years apart.
  #v(0.8em)
  #text(0.5em)[Search for MJML in Zed's extensions panel \ `github.com/pataruco/zed-mjml`]

  #speaker-note[
    The first thing I shipped was an HTML email. The latest is a Rust program that checks HTML emails. Same problem, _TODO N_ years apart, different tools.

    It started on 7 March 2026 as an experiment. It has been version 0.3.1 since 4 October. If you use Zed, search for MJML in the extensions panel. If you still build email templates, I'd love your bug reports.
  ]
]
