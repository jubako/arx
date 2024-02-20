Zip2Arx
=======

`zip2arx` is a command line tool to convert zip archive to arx format.

[Arx](https://crates.io/crates/arx) is a new file archive format.

Install zip2arx
===============

Binaries for Windows, MacOS and Linux are available for [every release](https://github.com/jubako/arx/releases).
You can also install zip2arx using Cargo:

```
cargo install zip2arx arx
```

You will need `arx` to read the archive.


Use zip2arx
===========


Convert a zip archive
---------------------

Creating an archive is simple :


```
zip2arx -o foo.arx foo.zip
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
