# GPUI Component basic examples

This folder contains basic examples of how to use the GPUI Component library. Each example demonstrates a specific feature or functionality of the library.

Each Rust package runs with `cargo run -p <package-name>`. The larger examples
reuse the `story` gallery components as a normal dependency, so running them does
not enable the gallery's test-support development dependency.

| Example | Command |
| --- | --- |
| Editor | `cargo run -p example-editor` |
| Brush | `cargo run -p example-brush` |
| Dock | `cargo run -p example-dock` |
| HTML | `cargo run -p example-html` |
| Large text | `cargo run -p example-large-text` |
| Markdown | `cargo run -p example-markdown` |
| Streaming Markdown | `cargo run -p example-stream-markdown` |
| Tiles | `cargo run -p example-tiles` |

Shared sample documents live in `fixtures/`.

## Contributing

Feel free to contribute more examples to this folder!

If you have a specific use case or feature you'd like to demonstrate, please create a new example file and submit a pull request. We will happy to merge it into the repository.

When creating a new example, please follow these guidelines:

1. Keep 1 example just doing 1 thing for more clarity.
2. Testing the example to ensure it works as expected.
3. Write some comment at some key parts of the code to explain what it does.
4. Following the code style and name style used in the existing examples or in entire of GPUI Component.
