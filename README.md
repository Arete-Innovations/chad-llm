# chad-llm

CLI interface for OpenAI API.

<a href="https://asciinema.org/a/QipAmCRYIy4ZIbJZfKMoLOgNC" target="_blank"><img src="https://asciinema.org/a/QipAmCRYIy4ZIbJZfKMoLOgNC.svg" /></a>

## Building

If you wish to run this program on Windows, you need a terminal that supports
ANSI escape codes and raw mode.

```
# Clone the repository
git clone --depth 1 https://github.com/ZmoleCristian/chad-llm.git chad-llm
# Change directory
cd chad-llm
# Build and run with cargo
cargo build --release
```

## Running

First, a value for the `OPENAI_API_KEY` environment variable is required. Get
one from the [OpenAI platform](https://platform.openai.com/settings/organization/api-keys).

Make sure you have credits as well. You can set it up to auto-add credits.

Finally, run the program: `./target/release/chad-gpt`.

## License

This project is licensed under the BSD-3-Clause license. For more information
read the [license](./LICENSE) file.

