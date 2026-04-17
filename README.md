# veneer

A documentation engine. Currently ships one command: `veneer extract`, which reads a CLI binary's `--help` output and emits Astro-compatible MDX reference pages plus a sidebar JSONL.

The component-documentation pipeline (framework-less Web Component previews from React/Vue/Solid source) lives in `veneer-adapters` and is being rewired into a registry-first flow. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/rafters-studio/veneer/main/install.sh | bash
```

Or from source:

```bash
cargo install --path crates/veneer
```

## Extract CLI docs

```bash
veneer extract \
  --project ./path/to/your-cli \
  --binary ./path/to/your-cli/target/release/your-cli \
  --output ./docs \
  --layout "@/layouts/Docs.astro"
```

Produces:

- `docs/reference/**/*.mdx` — one page per command, with flag tables and usage
- `docs/sidebar.jsonl` — nav data the consuming site imports
- `docs/concepts/*.mdx` — editorial skeleton pages (getting-started, architecture, etc.)

The `--layout` value is written into MDX frontmatter as `layout:` so Astro resolves it.

## Workspace

```
crates/
├── veneer            CLI binary (extract command)
├── veneer-adapters   JSX → Web Component transformation, scope_css
└── veneer-docs       CLI help parser, MDX skeletons, sidebar generation
```

## License

MIT
