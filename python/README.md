# libarx: A Python Wrapper for the Arx Archive Library

`libarx` is a Python library providing a user-friendly interface to interact with Arx archives.
Arx is a high-performance, content-addressable archive format.
This library allows you to easily create, read, and manipulate Arx archives from your Python code.

## Key Features

* **Create Arx archives:**  Easily create new Arx archives from files and directories.
* **Read Arx archives:** Efficiently read and extract data from existing Arx archives.
* **Iterate over archive contents:**  Traverse the archive's tree structure and access individual entries.
* **Access individual entries:** Directly access specific files within the archive by path.
* **Stream-based access:** Read file contents as streams, optimizing memory usage for large files.


## Installation

Install it using pip:

```bash
pip install libarx
```

Our [PyPI wheels](https://pypi.org/project/libarx/) bundle the Rust libarx and are available for the following platforms:

- macOS for `x86_64`
- GNU/Linux for `x86_64`
- Windows for `x64`

Wheels are available for CPython only.

Users on other platforms can install the source distribution (see [Building](#Building) below).

## Usage Examples

### Creating an Archive

```python
import libarx

# Create a new archive (replace with your desired output path)
archive_path = "my_archive.arx"
with libarx.Creator(archive_path) as creator:
    # Add files and directories to the archive
    creator.add("path/to/file1.txt")  #adds file
    creator.add("path/to/directory") #adds directory and its content recursively
```

### Reading an Archive

```python
import libarx

archive_path = "my_archive.arx"

# Open the archive
arx = libarx.Arx(archive_path)

def iterate(iterable, root=""):
    for entry in iterable:
        path = root + "/" + entry.path
        print(f"Entry: {path}")
        if entry.is_file():
            content_stream = entry.get_content()
            print(f"  Content size: {content_stream.size()} bytes")
            content = content_stream.read(min(100, content_stream.size()))
            print(f"  Content: {content}")
        elif entry.is_dir():
            print(f"  Nb Children: {entry.nb_childen()}")
            loop_on_entry_generator(entry)
        elif entry.is_link():
            print(f"  Link to {entry.get_target()}")

# Walk the entries in the archive
iterate(arx)

# Access a specific entry
specific_entry = arx.get_entry("path/to/file1.txt")
assert specific_entry.path == "file1.txt"
assert entry.parent.path == "to"
assert entry.parent.parent.path == "path"
assert entry.parent.parent.parent == None

#Extract the archive
arx.extract("extracted/archive/path")
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## Sponsoring

I ([@mgautierfr](https://github.com/mgautierfr)) am a freelance developer. All jubako projects are created in my free time, which competes with my paid work.
If you want me to be able to spend more time on Jubako projects, please consider [sponsoring me](https://github.com/sponsors/jubako).
You can also donate on [liberapay](https://liberapay.com/jubako/donate) or [buy me a coffee](https://buymeacoffee.com/jubako).

## License

This project is licensed under the MIT License - see the [LICENSE-MIT](LICENSE-MIT) file for details.
