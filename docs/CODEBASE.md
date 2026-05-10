This will mainly focus on what the
codebase is like for any potential contributors.
Feel free to ask any questions on [Discord](https://discord.gg/bWqRaSXar5).

# Crate Structure

- `quantum_launcher` - The **GUI frontend**
- `ql_instances` - Instance management, authentication and launching
- `ql_servers` - A self-hosted server management system (incomplete)
- `ql_mod_manager` - **Mod management** and installation
- `ql_packager` - Code related to packaging/importing instances
- `ql_core` - Core utilities and shared code
- `ql_java_handler` - A library to **auto-install** and provide **Java** runtimes

# UI Pattern

The architecture of the launcher is based on the
Model-View-Controller pattern (AKA the thing used in iced).

- The `Launcher` struct holds the state of the app.
- `view(&Launcher)` renders the app's view based on the current state.
- `update(&mut Launcher)` processes messages and updates the state accordingly.
- The `state::State` enum determines which menu is currently open.

So it's a back-and-forth between `Message`s coming from interaction,
and code to deal with the messages in `update()`.

# Helpers/Patterns

## 1) Logging

Use `info!()`, `pt!()`, `err!()` for launcher logs:

```txt
[info] Installing something
- Doing step 1...
- Doing step 2...
- Another point message
[error] 404 when downloading library at (https://example.com/downloads/something.jar), skipping...
```

Log only useful messages that aid troubleshooting.
When a user sends logs for a broken launcher,
the message should help identify the issue.

- **Info**: Big-picture updates.
- **Pt (point)**: Small, step-by-step details.
- **Err** Errors. Use Result<T, E> for non-recoverable errors,
  err!() for recoverable warnings.

There is no warn!() macro, as non-fatal errors are logged with err!(),
and fatal ones should be returned directly.

## 2) IO

Prefer async code for filesystem and network operations.
This can be relaxed occasionally, but it's generally recommended.

Use `tokio::fs` for filesystem tasks and
`ql_core::file_utils::download_file_to_*` for networking.

Explore `ql_core::file_utils` for useful utilities, or check `cargo doc`.
A common pattern is importing `ql_core::file_utils` and calling `file_utils::*` manually.

## 3) Errors

Return fatal errors as Result<T, E> for those that can’t be ignored.
Create custom error enums for specific tasks, like `ForgeInstallError`, `GameLaunchError`, etc.

Avoid `Box<dyn Error>`, instead convert errors to String using `.strerr()`
(via `ql_core::IntoStringError trait`).

> ### Why?
> `iced` requires that all messages be cloneable. `Box<dyn Error>`
> can't be cloned but `String` can.

Use `thiserror` with `#[derive(Debug, thiserror::Error)]` for your error types.
All errors must implement `Debug`, `thiserror::Error`, and `Display`.
Use `#[from]` and `#[error]` as needed:

```rust
use thiserror::Error;

const MY_ERR_PREFIX: &str = "while doing my thing\n:";

#[derive(Debug, Error)]
enum MyError {
    // Add context for third-party errors
    #[error("{MY_ERR_PREFIX}while extracting zip:\n{0}")]
    Zip(ZipError),

    // But not for QuantumLauncher-defined errors
    #[error("{MY_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{MY_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),

    #[error("{MY_ERR_PREFIX}no valid forge version found")]
    NoForgeVersionFound,
}
```

For user-facing errors, make them clear and friendly, e.g.:

```txt
while doing my thing:
while installing forge:
while extracting installer:
Zip file contains invalid data!
```

For common errors (IO, network), offer simple troubleshooting steps
and move technical details out of the way (but don't hide them!).
Capitalization is flexible, use what feels best.

## 4) Error Magic

Here are some handy error handling methods **that can be called on `Result<T, E>`**:

### `.path(your_path)` (from `ql_core::IntoIoError` trait)

Converts `std::io::Error` into a nicer `ql_core::IoError`.

```rust
tokio::fs::write(&path, &bytes).await.path(path)?;
```

### `.json(original_string)` (from `ql_core::IntoJsonError`)

For adding context when parsing JSON **strings** into structs. Use on `serde_json` errors.

### `.json_to()` (from `ql_core::IntoJsonError`)

For use when converting **structs** into json strings.

### `.strerr()` (from `ql_core::IntoStringError`)

Converts any error into `Result<T, String>`,
useful for "dynamic" or "generic" errors.
Required for async functions called by the GUI.

**More docs coming in the future...**
