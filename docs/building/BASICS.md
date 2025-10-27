This guide will show you how to compile **QuantumLauncher**
from source code. It's designed to be helpful for beginners and walks
through the entire process.

# Index

- [Prerequisites](#prerequisites)
- [Getting the source code](#1-getting-the-source-code)
    - [Git](#git-recommended)
    - [Zip](#alternate-method-zip)
- [Building](#2-building)
    - [Getting the Executable](#getting-the-executable)
    - [Release vs Debug](#release-vs-debug)

# Prerequisites:

Before you start, you will need to install
the [Rust programming language](https://rustup.rs/).

It's also a good idea to have `git` installed, although
it's not strictly necessary.

> TLDR; Here are the commands.
>
> ```sh
> git clone https://github.com/Mrmayman/quantumlauncher.git
> cd quantumlauncher
> cargo run
> ```
>
> Please read on for more information.

# 1) Getting the source code

## Git (recommended)

Using Git allows you to easily manage the source code,
fetch updates, and contribute to the project.

1. **Open the terminal** and navigate to your desired directory (maybe your projects folder)

```sh
cd path/to/folder
```

2. Clone the repository

```sh
git clone https://github.com/Mrmayman/quantumlauncher.git
cd quantumlauncher
```

To update the code in the future, run:

```sh
git pull
```

## Alternate method: Zip

If you prefer not to use Git,
you can download the source code as a ZIP file:

1. Go to the [QuantumLauncher repository](https://github.com/Mrmayman/quantumlauncher),
2. Click on the green **"Code"** button, then select **"Download ZIP"**
3. Extract the ZIP file, then open the terminal in the extracted folder

# 2) Building

Make sure you have Rust installed on your system.
You can get it from <https://rustup.rs>.

In your terminal, navigate to the folder where you
downloaded the source code. Run the following command
to compile and run the launcher:

```sh
cargo run
```

This command will build and run the launcher in debug mode.
Use this to test the app after making any changes.

## Getting the executable

If you want to just compile the launcher without running it, use:

```sh
cargo build
```

You can find the executable in the `target/debug/`
or `target/release/` folder. The executable will be called
The executable program will be called `quantum_launcher.exe`
on Windows, and `quantum_launcher` on other platforms.

## Release vs Debug

- **Debug** (default): Faster compilation, but the app is
  larger and less optimized. Use `cargo run` or `cargo build`
  to compile the launcher in debug mode

- **Release**: Optimized for performance, resulting in much
  smaller file size, but slower compilation times. To compile in release mode
  use `cargo run --release` or `cargo build --release`

Note: Release builds may take longer to compile because it performs
optimizations for an official release-ready build.
