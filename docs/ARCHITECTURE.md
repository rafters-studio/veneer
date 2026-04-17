# Architecture

## What veneer is

A documentation engine that emits MDX for consumption by an Astro site. One command today (`extract`) and a component-transformation library (`veneer-adapters`) that turns framework-specific JSX into framework-less Web Components.

## Workspace

```
crates/
├── veneer            CLI binary — entry, arg parsing, command dispatch
├── veneer-adapters   JSX → Web Component (React adapter), scope_css
└── veneer-docs       CLI --help parser, sidebar generation, MDX skeletons
```

### Dependency graph

```
veneer (CLI)
  ├── veneer-adapters
  └── veneer-docs
```

No cross-dependencies between the two library crates.

## veneer-adapters

Transforms framework-specific component source into vanilla Web Components so previews work in any site with zero runtime.

Responsibilities:

- **React adapter** (`react.rs`) — parses `.tsx` / `.classes.ts` via oxc, extracts variants/sizes/base classes, emits an ES6 class extending `HTMLElement` with Shadow DOM
- **scope_css** (`scope.rs`) — scopes Tailwind-generated CSS to a specific component's used classes, so a preview on a docs page cannot leak styles into the page shell
- **Registry discovery** (`registry.rs`) — walks a component source tree to find declared components
- **TS helpers** (`ts_helpers.rs`) — shared oxc utilities used by react.rs and registry.rs

## veneer-docs

Parses a clap binary's `--help` tree and generates MDX.

Responsibilities:

- **`cli_parser.rs`** — reads `--help` stdout, recursively walks subcommands, builds a typed `Command` tree with flags, usage, and required-flag detection
- **`reference.rs`** — renders the command tree into per-command MDX pages under `reference/`
- **`skeleton.rs`** — generates editorial MDX pages (getting-started, architecture, one per command group) with veneer-editorial slots
- **`sidebar.rs`** — produces a JSONL file the consuming Astro site imports to build nav

## veneer (CLI)

Thin clap wrapper. Dispatches to `commands::extract::run`.

## Data flow

```
CLI binary (any clap app)
    │
    └─ --help stdout
           │
           ▼
   ┌───────────────┐
   │ veneer-docs   │  parse, skeleton, sidebar
   └───────┬───────┘
           │
           ▼
   .mdx files + sidebar.jsonl
           │
           ▼
   Astro site consumes via layout + import
```

## Web Component pipeline (library, not yet wired into a command)

```
React source (.tsx + .classes.ts)
    │
    ▼
┌─────────────────┐
│ veneer-adapters │  parse, extract variants, emit WC class
└───────┬─────────┘
        │
        ▼
ES6 class extending HTMLElement
with Shadow DOM + scoped styles
```

The scope_css function takes the full Tailwind output CSS and the set of classes used by a given component, and returns a minimal stylesheet containing only rules that match those classes (plus their `@keyframes`, custom properties, and `@supports` / `@media` parents). This keeps Shadow DOM style sheets small and prevents cross-component bleed.

## What lives outside this repo

- **Target sites** — Astro sites (rafters.studio, runlegion.dev, huttspawn.com, gitpress.app) consume the MDX veneer emits. Veneer has no opinion on the site's layout components beyond the `--layout` frontmatter pointer.
- **Service design artifacts** — moved to a private premium repo.
- **Registry / component intelligence enrichment** — planned, not yet built. See issues #24–#27.

## Design invariants

- **Framework-less previews.** Any site should be able to render a veneer preview without running React, Vue, or Solid at runtime. Web Components are the contract.
- **No inference when data exists.** If intelligence (cognitive load, DO/NEVER, token usage) lives as structured data, read it. Don't guess from source.
- **MDX as the interchange format.** Astro is the rendering target. Emit MDX with layout pointers; let the site own layout, nav, and typography.
