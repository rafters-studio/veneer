# veneer

Veneer reads a rafters project and turns it into that project's docs, current with the code and maintained by nobody by hand. The same reading lets it check every use of rafters and compile composites into the project's own framework.

Veneer is being rebuilt. The previous Rust CLI is gone, and `@rafters/veneer` does not do anything yet. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for where the rebuild stands.

## Workspace

```
packages/veneer        @rafters/veneer, TypeScript, published to npm
crates/veneer-check    a scaffold with no dependents
crates/veneer-lsp      the language server
editors/               the editor packages, a scaffold so far
```

## Contributing

See [docs/DEVELOPER.md](docs/DEVELOPER.md).

## License

MIT
