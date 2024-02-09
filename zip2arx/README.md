Convert zip archive to arx
==========================

[Arx](https://crates.io/crates/arx) is a new file archive format.

Install zip2arx
---------------

```
cargo install zip2arx arx
```

You will need `arx` to read the archive.


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
