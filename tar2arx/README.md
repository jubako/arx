Tar2Arx
=======

`tar2arx` is a command line tool to convert tar archive to arx format.

[Arx](https://crates.io/crates/arx) is a new file archive format.

Install tar2arx
===============

Binaries for Windows, MacOS and Linux are available for [every release](https://github.com/jubako/arx/releases).
You can also install tar2arx using Cargo:

```
cargo install tar2arx arx
```

You will need `arx` to read the archive.


Use tar2arx
===========


Convert a tar archive
---------------------

Creating an archive is simple :


```
tar2arx -o foo.arx foo.tar.gz
```

Read arx archive
----------------

See the arx documentation for full command.

- List the content of the archive

```
arx list foo.arx | less
```

- Extract the archive

```
arx extract -f foo.arx -C my_out_dir
```

- Extract only one file


```
arx dump foo.arx my_directory/path/to/my_file > my_file
```

- Mounting the archive

On linux and macOs, you can mount the archive using fuse.

```
mkdir mount_point
arx mount foo.arx mount_point
```

`arx` will be running until you unmount `mount_point`.
