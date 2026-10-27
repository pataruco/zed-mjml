// Talk: From HTML emails to Rust. How I built MJML for Zed.
//
// Build the slides:        typst compile --font-path talk/fonts talk/main.typ talk/slides.pdf
// Build slides + notes:    typst compile --font-path talk/fonts --input notes=true talk/main.typ talk/notes.pdf
//
// Styled after awesome.red-badger.com: its colours come from the site's _variables.scss and
// the logos from its assets. The spoken script lives in the #speaker-note blocks under each
// slide. Things to fill in before the day are marked TODO.

#import "@preview/touying:0.8.0": *
#import themes.simple: *
#import "@preview/fletcher:0.5.8" as fletcher: diagram, node, edge

#let with-notes = sys.inputs.at("notes", default: "false") == "true"

// Fonts. The site uses Proxima Nova for text and Sentinel for headlines. Both are licensed to
// the website only, so this deck ships open stand-ins in fonts/ (SIL Open Font License):
// Figtree for Proxima Nova and Zilla Slab for Sentinel. If you have the brand fonts installed,
// put them first: ("Proxima Nova", "Figtree") and ("Sentinel", "Zilla Slab").
#let sans = "Figtree"
#let slab = "Zilla Slab"
#let mono = "DejaVu Sans Mono"

// Colours, from the site's _variables.scss
#let bg = rgb("#F8F8F8") // page background
#let ink = rgb("#111111") // text
#let red = rgb("#FC1D42") // brand colour, used for links on the site
#let grey = rgb("#969696") // post meta
#let grey-dark = rgb("#565656") // the rule above the site header
#let border = rgb("#CCCCCC")
#let code-bg = rgb("#EEEEFF") // code blocks on the site
#let sandboxed = red.lighten(85%) // parts that run inside Zed's WebAssembly sandbox
#let native = white // the one part that runs as a native process

#show: simple-theme.with(
  aspect-ratio: "16-9",
  primary: red,
  footer: image("assets/logo-red-badger.png", height: 1.2em),
  subslide-preamble: block(
    below: 1.1em,
    text(
      font: slab,
      size: 1.45em,
      weight: "bold",
      tracking: -0.02em,
      fill: ink,
      utils.display-current-heading(level: 2),
    ),
  ),
  config-page(
    fill: bg,
    background: place(top + left, rect(width: 100%, height: 3pt, fill: grey-dark)),
  ),
  config-common(
    show-notes-on-second-screen: if with-notes { right } else { none },
    // The notes panel defaults to 24pt, which clips the longer notes. 17pt fits every note.
    notes-fn: (self: none, ..args) => notes(
      self: self,
      note-setting: note => pad(x: 36pt, y: 14pt, text(size: 17pt, note)),
      ..args,
    ),
  ),
  config-info(
    title: [From HTML emails to Rust],
    subtitle: [MJML for Zed],
    author: [Pedro Martin],
    date: datetime.today(),
  ),
)

#set text(font: sans, fill: ink)
#show raw: set text(font: mono)
#show raw.where(block: true): it => block(
  fill: code-bg,
  stroke: border + 0.5pt,
  inset: 10pt,
  radius: 3pt,
  width: 100%,
  it,
)

// A small grey caption above a snippet or picture, like the site's post meta.
#let caption(it) = text(size: 0.65em, fill: grey, it)

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
  radius: 3pt,
  fill: white,
  stroke: (paint: border, dash: "dashed", thickness: 1.5pt),
  align(center + horizon, text(size: 0.7em, fill: grey, what)),
)

// A red marker at the top of the notes of a slide you can skip.
#let cut = text(fill: red, weight: "bold")[Short version: skip this slide if short on time.]

// Small text for diagram nodes.
#let cell(body) = text(0.55em, body)

// ---------------------------------------------------------------------------

#title-slide[
  #block(width: 100%)[
    #set align(left)
    #image("assets/logo-awesome.png", height: 2.6cm)
    #v(0.4em)
    #text(font: slab, size: 2em, weight: "bold", tracking: -0.02em)[MJML for Zed]
    #v(0.9em)
    #text(size: 0.65em, fill: grey)[Pedro Martin]
  ]
]


// ---------------------------------------------------------------------------

== MJML: you write MJML, email clients get HTML tables

#grid(
  columns: (1fr, 1.4fr),
  column-gutter: 1em,
  [#caption[You write] #snippet("hello.mjml", lang: "html")],
  [
    #caption[Email clients get]
    #block(height: 5.8cm, clip: true, radius: 3pt, snippet("compiled.html", lang: "html", size: 8.5pt))
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

#grid(
  columns: (1fr, 1fr),
  column-gutter: 1em,
  [#image("assets/good.png")],
  [#image("assets/bad.png")],
)

#v(0.4em)
#align(center, caption[Written in Rust · draws its interface on the GPU · extensions are Rust compiled to WebAssembly])

#speaker-note[
  Zed is a code editor. It is written in Rust by the people who built Atom and tree-sitter. It is fast because it draws its interface on the GPU, the way a game does.

  Two things matter for this talk. Zed extensions are written in Rust and compiled to WebAssembly, not JavaScript. And Zed is the editor I use every day. It didn't understand `.mjml` files, so I had an itch to scratch.
]

// ---------------------------------------------------------------------------

== Tree-sitter turns your file into a tree

#grid(
  columns: (1fr, 1fr),
  column-gutter: 1em,
  row-gutter: 0.6em,
  [#caption[Your file] #snippet("tree-input.mjml", lang: "html")],
  grid.cell(rowspan: 2)[#caption[The tree, simplified] #snippet("tree.txt", size: 11pt)],
  [#caption[A query from `languages/mjml/highlights.scm`] #snippet("highlights.scm", lang: "scm", size: 11pt)],
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
  node-corner-radius: 3pt,
  node-inset: 10pt,
  node-shape: rect,
  spacing: (9em, 1em),
  node((0, 0), [*Editor* \ #text(0.7em)[Zed]], name: <ed>, fill: white, stroke: ink + 1pt),
  node((1, 0), [*Language server* \ #text(0.7em)[`mjml-lsp`]], name: <ls>, fill: native, stroke: red + 1.5pt),
  edge(<ed>, <ls>, "->", text(0.55em, align(center)[document opened or changed \ what goes here? what is this? how do I fix it?]), bend: 24deg, stroke: ink + 0.8pt),
  edge(<ls>, <ed>, "->", text(0.55em, align(center)[errors and warnings \ completions, hover docs, quick fixes]), bend: 24deg, stroke: ink + 0.8pt),
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

#let pass(number, title, body) = block(
  fill: white,
  stroke: border + 0.5pt,
  inset: 16pt,
  radius: 3pt,
  width: 100%,
  height: 5.8cm,
)[
  #text(0.9em, weight: "bold")[#text(fill: red)[#number] #title]
  #v(0.5em)
  #text(0.62em, body)
]

#grid(
  columns: (1fr, 1fr),
  column-gutter: 1.2em,
  pass[1.][Tag scanner + rules table][
    Nesting: `mj-text` inside `mj-section` \
    Required attributes: `src` on `mj-image` \
    Unknown tags, with "did you mean?" \
    A second `mj-head` or `mj-body`
  ],
  pass[2.][mrml, a Rust port of MJML][
    Unclosed tags and malformed markup \
    Follows `mj-include`, so a template split into partials validates as a whole
  ],
)


#speaker-note[
  #cut

  mjml-lsp is the Rust binary in this repo, about 4,400 lines including tests. It checks a document in two passes.

  First, my own scanner walks the tags and applies the MJML rules: nesting, required attributes, unknown tags with "did you mean", duplicate heads and bodies. The rules live in one table. Validation, completion and hover all read from it, so adding a component is one entry.

  Second, mrml, a Rust port of MJML, parses the document and catches structural problems: unclosed tags, malformed markup. It also follows `mj-include`, so a template split into partials validates as a whole.
]

// ---------------------------------------------------------------------------

== WebAssembly: one binary, any host

#align(center, diagram(
  node-stroke: grey-dark + 0.8pt,
  node-fill: white,
  node-corner-radius: 3pt,
  node-inset: 9pt,
  node-shape: rect,
  spacing: (3em, 0.35em),
  node((0, 1.5), cell[Rust, C, C++, Go, ...], name: <src>),
  node((1, 1.5), cell[*`.wasm` binary*], name: <bin>, fill: sandboxed, stroke: red + 1pt),
  node((2, 0), cell[Browser], name: <t0>),
  node((2, 1), cell[Zed], name: <t1>),
  node((2, 2), cell[Node.js], name: <t2>),
  node((2, 3), cell[Cloudflare Workers], name: <t3>),
  edge(<src>, <bin>, "->", text(0.5em)[compile]),
  edge(<bin>, <t0>, "->"),
  edge(<bin>, <t1>, "->"),
  edge(<bin>, <t2>, "->"),
  edge(<bin>, <t3>, "->"),
))
#v(0.6em)
#let property(title, body) = block(fill: white, stroke: border + 0.5pt, radius: 3pt, inset: 10pt, width: 100%)[
  #text(0.7em, weight: "bold", fill: red, title) \
  #text(0.56em, body)
]
#grid(
  columns: (1fr, 1fr, 1fr, 1fr),
  column-gutter: 0.6em,
  property[Fast][Near-native speed, predictable performance],
  property[Safe][Runs in its own sandboxed memory, isolated from the host],
  property[Portable][The same file runs anywhere there is a Wasm runtime],
  property[Language-agnostic][Compile Rust, C, C++, Go and more to it],
)

#speaker-note[
  Let me zoom out, because WebAssembly matters twice in this project.

  WebAssembly, Wasm for short, is a binary format you compile to. You compile Rust, C or Go into a `.wasm` file, and any host with a Wasm runtime can run it: a browser, Node.js, Cloudflare Workers, or an editor like Zed.

  Four properties make it interesting. Fast: near-native speed. Safe: it runs in its own sandboxed memory, isolated from the host. Portable: the same file runs anywhere there is a runtime. Language-agnostic: Rust, C, C++, Go and more.

  Safety is the one that matters here. A host can load a stranger's module without worrying about it touching the file system, the network, or memory outside its own. That is why browsers and code editors both adopted it as their extension format.

  I first heard of it from a colleague, Stu, who shared a Slack message on 26 December 2019: the W3C had made WebAssembly an official web standard, the fourth language of the web after HTML, CSS and JavaScript. It planted a seed, but it felt like something for game engines, not for me.
]

// ---------------------------------------------------------------------------

== Wasm is not one thing. It is a spectrum.

#let encounter(role, title, body) = block(
  fill: white,
  stroke: border + 0.5pt,
  radius: 3pt,
  inset: 14pt,
  width: 100%,
  height: 7.9cm,
)[
  #text(0.6em, fill: red, weight: "bold", role) \
  #text(0.85em, weight: "bold", title)
  #v(0.4em)
  #text(0.58em, body)
]
#grid(
  columns: (1fr, 1fr, 1fr),
  column-gutter: 0.8em,
  encounter[Wasm does the work][A passport reader in the browser][A know-your-customer model reads a passport photo on the user's machine. The Wasm runs the inference. JavaScript runs the camera and the UI.],
  encounter[Wasm is the brain][Stained Wall, a Firefox extension][Rust decides, TypeScript executes. The Wasm turns a URL into a plan. TypeScript carries the plan out with the browser's APIs.],
  encounter[Wasm is the hand][MJML for Zed][The Wasm downloads a binary and steps aside. A native language server does all the work.],
)
#v(0.5em)
#rect(width: 100%, height: 6pt, radius: 3pt, fill: gradient.linear(red, luma(215)))
#grid(
  columns: (1fr, 1fr),
  caption[Heavy: the Wasm is the program],
  align(right, caption[Thin: the Wasm is a wrapper]),
)

#speaker-note[
  #cut

  I was wrong about that. Three encounters, each teaching me something different.

  First, a know-your-customer project. The browser had to read a passport or driving licence. The vendor's SDK shipped a machine learning model compiled to Wasm. It ran the inference on the user's machine, without sending the image to a server. JavaScript ran the camera and the UI. Lesson: Wasm is not a faster JavaScript. It brings tools from other languages into places that only spoke JavaScript.

  Second, Stained Wall, a Firefox extension I built to keep news from home readable. The principle: Rust decides, TypeScript executes. The Rust engine, compiled to Wasm, takes a URL and returns a plan as JSON. TypeScript carries the plan out with the browser's APIs. The Wasm is the brain.

  Third, this project. The Wasm is the hand: it downloads a binary and steps aside. The native language server does all the work.

  So Wasm is not one thing. It is a spectrum: from heavy modules that do the computation, to thin wrappers that exist only to satisfy a platform's extension format.
]

// ---------------------------------------------------------------------------

== WebAssembly is a sealed box. WASI is the standard set of doors.

#let door(name, fill: luma(232), stroke: grey) = box(
  fill: fill,
  inset: (x: 8pt, y: 5pt),
  radius: 3pt,
  stroke: stroke + 0.8pt,
  text(0.5em, name),
)
#let sealed(title, doors) = [
  #align(center, caption(title))
  #v(0.3em)
  #rect(
    width: 100%,
    height: 5.5cm,
    stroke: ink + 4pt,
    radius: 6pt,
    fill: white,
    inset: 12pt,
  )[
    #v(0.3cm)
    #align(center, rect(fill: bg, stroke: grey-dark + 1pt, radius: 3pt, inset: 14pt)[
      your code \ #text(0.6em, fill: grey)[compiled to Wasm]
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
        door(fill: sandboxed, stroke: red)[GitHub release],
        door(fill: sandboxed, stroke: red)[download],
        door(fill: sandboxed, stroke: red)[run command],
      ),
    )
  ],
)
#v(0.3em)
#align(center, caption[Grey doors come from WASI. Red doors are what Zed adds through its extension interface.])

#speaker-note[
  This is the part that confused me the most, so here is the version I wish someone had told me.

  WebAssembly, or Wasm, is a portable binary format. You compile Rust, C or Go to it, and it runs at near-native speed on any platform, inside a sandbox. The sandbox is the whole point. A Wasm module can't open a file, make a network call, or even read the clock. It can only call functions the host chooses to hand it.

  That is safe, but a program that can't touch anything isn't useful. So WASI, the WebAssembly System Interface, is a standard set of those functions: files, environment variables, clocks, random numbers, standard input and output. The host grants each one as a capability. Zed gives my extension its own folder and nothing else.

  Short version: Wasm is the engine. WASI is the standard sockets you plug it into.

  In the browser you never notice this, because JavaScript is the host and it hands the module every door it needs. Outside the browser there is no JavaScript. WASI fills that gap, so a module can run in Wasmtime inside Zed, on a server, or at the edge.
]

// ---------------------------------------------------------------------------


== Releases are automated. Commit messages decide the version.

#v(0.3em)
#align(center, diagram(
  node-stroke: grey-dark + 0.8pt,
  node-fill: white,
  node-corner-radius: 3pt,
  node-inset: 9pt,
  node-shape: rect,
  spacing: (1em, 0.5em),
  node((0, 0), cell[Conventional commits land on `main`]),
  edge("->"),
  node((0, 1), cell[release-please keeps a release pull request open]),
  edge("->"),
  node((0, 2), cell[Merging it creates the tag `zed-mjml-v0.3.1` and a GitHub release]),
  edge("->"),
  node((0, 3), cell[The Deploy workflow builds 3 binaries and attaches them to the release], fill: sandboxed, stroke: red + 1pt),
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

#focus-slide(config: config-page(background: none))[
  #text(font: slab, weight: "bold", tracking: -0.02em)[Same problem, 12 years apart.]
  #v(0.8em)
  #text(size: 0.5em)[Search for MJML in Zed's extensions panel \ `github.com/pataruco/zed-mjml`]
  #v(0.6em)
  #text(size: 0.4em)[More on Wasm: `pataruco.dev/blog/wasm-beyond-the-browser-and-back-again`]
  #v(0.6em)
  #text(size: 0.4em)[Official Zed extension page: `https://zed.dev/extensions/mjml`]

  #speaker-note[
    The first thing I shipped was an HTML email. The latest is a Rust program that checks HTML emails. Same problem, _TODO N_ years apart, different tools.

    It started on 7 March 2026 as an experiment. It has been version 0.3.1 since 4 October. If you use Zed, search for MJML in the extensions panel. If you still build email templates, I'd love your bug reports.
  ]
]
