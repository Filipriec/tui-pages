# Tui-pages

> **Origin:** Originally developed for TUI accounting system. This crate was extracted and optimized from it using AI(thank you claude, chatgbt and minimax).

## Docs:
```
mdbook serve mdbook --open
```

You want to create complex app in tui? Me the same. And I was stuck with god object mutating shared reference(I know, skill issues). But it happened. So I had to rewrite the whole architecture. And I did. Successfuly.   

This crate it that architecture. Asked AI to generalize so that you can also use it ;)

### How it worked before the generalization:
<img src="docs/full_system_legacy.mermaid.svg" width="600">


## Architecture

See [`docs/architecture/architecture.md`](docs/architecture/architecture.md)
for the full design, flow diagrams, and the primitive layer.
