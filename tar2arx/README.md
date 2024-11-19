# tar2arx: Tar to Arx Archive Converter

`tar2arx` is a command-line tool that converts tar archives to the Arx archive format.
Arx ([https://crates.io/crates/arx](https://crates.io/crates/arx)) offers a modern and performant alternative to traditional archive formats.
This tool streamlines the process of migrating existing tar archives to the benefits of Arx.

## Installation

### Using Cargo

The recommended installation method is using Cargo, Rust's package manager:

```bash
cargo install tar2arx arx
```

This command will install both `tar2arx` and its dependency, the `arx` library, which is necessary for reading and manipulating Arx archives.


### Pre-built Binaries

Pre-built binaries for Windows, macOS, and Linux are available for each release on the [GitHub releases page](https://github.com/jubako/arx/releases). Download the appropriate binary for your operating system and place it in your system's PATH.

## Usage Examples

**Converting a Local Tar Archive:**

```bash
tar2arx -o output.arx input.tar.gz
```

This command converts `input.tar.gz` to `output.arx`.


**Converting a Remote Tar Archive:**

```bash
tar2arx -o remote.arx https://example.com/data.tar.gz
```

This converts a remote tar archive from the specified URL (requires the `http` feature, which is enabled by default).


**Working with Arx Archives (using the `arx` command):**

After converting to Arx, use the `arx` command to interact with the created archive.  Here are some examples:

* **List archive contents:**
  ```bash
  arx list output.arx | less
  ```

* **Extract the archive:**
  ```bash
  arx extract output.arx -C my_output_directory
  ```

* **Extract a single file:**
  ```bash
  arx dump output.arx path/to/my/file.txt my_file.txt
  ```

* **Mount the archive (Linux/macOS):**
  ```bash
  mkdir mount_point
  arx mount output.arx mount_point
  ```
  Remember to unmount the archive using `umount mount_point` when finished.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## Sponsoring

I ([@mgautierfr](https://github.com/mgautierfr)) am a freelance developer. All jubako projects are created in my free time, which competes with my paid work.
If you want me to be able to spend more time on Jubako projects, please consider [sponsoring me](https://github.com/sponsors/jubako).
You can also donate on [liberapay](https://liberapay.com/jubako/donate) or [buy me a coffee](https://buymeacoffee.com/jubako).

## License

This project is licensed under the MIT License - see the [LICENSE-MIT](LICENSE-MIT) file for details.
