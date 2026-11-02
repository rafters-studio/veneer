# veneer

Veneer reads a rafters project and turns it into that project's docs, current with the code and maintained by nobody by hand. The same reading lets it check every use of rafters and compile composites into the project's own framework.

Veneer is being rebuilt. The previous Rust CLI is gone, and `@rafters/veneer` does not do anything yet. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for where the rebuild stands.

## Workspace

```
packages/veneer        @rafters/veneer, TypeScript, published to npm
crates/veneer-check    the shared check core, a scaffold so far
crates/veneer-lsp      the language server, a scaffold so far
editors/               the editor packages, a scaffold so far
```

## Contributing

See [docs/DEVELOPER.md](docs/DEVELOPER.md).

## License

MIT
