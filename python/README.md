# Libarx

`libarx` module allows you to read and write Arx
archives in Python. It provides a shallow python
interface on top of the [Rust `libarx` library](https://github.com/jubako/arx).

## Installation

```sh
pip install libarx
```

Our [PyPI wheels](https://pypi.org/project/libarx/) bundle the Rust libarx and are available for the following platforms:

- macOS for `x86_64`
- GNU/Linux for `x86_64`
- Windows for `x64`

Wheels are available for CPython only.

Users on other platforms can install the source distribution (see [Building](#Building) below). 


## Usage

### Read an Arx archive

```python
from libarx import Arx

archive = Arx("my_archive.arx")
entry = arx.get_entry("foo/bar")

print(f"Entry (idx: {entry.idx}) name is {entry.path}")
if entry.is_file():
    print("Entry is a file").
    content = entry.get_content()
    print(content)
elif entry.is_link():
    print(f"Entry is a link pointing to {entry.get_target()}")
else:
    print("Entry is a directory.")
    print(f"children are ranged from {entry.first_child()} to {entry.first_child()+entry.nb_children()}")

# We can also iterate on entries in arx
def iterate(iterable, root=""):
    for entry in iterable:
        path = root + "/" + entry.pathi
        if entry.is_file():
            content = entry.get_content()[..512]
            print(f"File {path} : {content}")
        elif entry.is_link():
            print(f"Link {path} -> {entry.get_target()}")
        else:
            print(f"Dir {path}")
            iterate(entry, path)

iterate(arx)        
    

# Arx archive can simply be extracted with :
arx.extract("target/directory/where/to/extract")
```

### Create an Arx archive

```python
from libarx import Creator

with Creator("my_archive.arx") as creator:
  creator.add("path/to/entry/to/add", recursive=(False or True))
```

## Building

Python `libarx` is compiled using [maturin](https://www.maturin.rs/)

- Install rust (https://www.rust-lang.org/learn/get-started)
- Install python
- Install maturin : `pip install maturin`
- Build everything : `maturin build`

## License

[MIT](https://mit-license.org/)


## Support

libarx, Arx and all Jubako project is developed on my spare time. If you liked it, please
consider sponsor me. At your convinence: (Github)[https://github.com/sponsors/jubako],
(liberapay)[https://liberapay.com/jubako/donate] or (buy me a coffe)[https://buymeacoffee.com/jubako]
