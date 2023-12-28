What is arx
===========

Arx is a file archive format based on the
[jubako container format](https://github.com/jubako/jubako).

It allow you to create, read, extract file archive (as zip or tar does).

Arx (and Jubako) is in active development.
While it works pretty well, I do not recommand to use it to do backups.
However, you can use it to transfer data or explore archives.


How it works
============


Jubako is a versatile container format, allowing to store data, compressed or
not in a structured way. It main advantage (apart from its versability) is
that is designed to allow quick retrieval of data fro the archive without
needing to uncompress the whole archive.

Arx use the jubako format and create arx archive which:
- Store file's data compressed.
- Store files using a directory/tree structure.
- Can do random access on the arx archive to get a specific files
- Allow to mount the archive to explore and use (read only) the files in the
  archive without decompressing it.

Try arx
=======

Install arx
-----------

```
cargo install --git https://framagit.org/jubako/arx
```


Create an archive
-----------------

Creating an archive is simple :


```
arx create --file my_archive.arx -1r my_directory
```

It will one file : `my_archive.arx`, which will contains the `my_directory` directory.


Extract an archive
------------------

Extracting (decompressing) an archive is done with :

```
arx extract -f my_archive.arx -C my_out_dir
```


Listing the content of an archive
---------------------------------

You can list the content of (the list of files in) the archive with :

```
arx list my_archive.arx
```

And if you want to access to the content of only one file :

```
arx dump my_archive.arx my_directory/path/to/my_file > my_file
```

Mounting the archive
--------------------

On linux, you can mount the archive using fuse.

```
mkdir mount_point
arx mount my_archive.arx mount_point
```

`arx` will be running until you unmount `mount_point`.

Converting a zip archive to an arx archive
------------------------------------------

```
zip2ar -o my_archive.arx my_zip_archive.zip
```

Converting a tar archive to an arx archive
------------------------------------------

The tool `tar2arx` works on uncompressed tar file.
Most of the time, you will have a copmressed tar file (`tar.gz`, ...)
`tar2arx` expect the tar content to be given on its standard input.

```
gzip -d my_archive.tar.gz | tar2arx -o my_archive.arx
```



Performance
===========


The following compare the performance of arx and tar, depending of different
use cases.

Arx is compressing the content using zstd.
By default, most of the time, tar is compressed using gz.

To compress create a tar archive with the same compression level, we use
`tar c <directory> | zstd -TN -o <archive>.tar.zst`
(N of `-TN` being the number of thread to use when compression).
I have use `-T8` in my tests.

Tests has been done on different data sets :
- the whole linux kernel (linux-5.19)
- the drivers directory in linux kernel
- the document directory in the linux kernel

Listing time corresponds to `arx list archive.arx > listing.txt` or
`tar --list -f archive.tar.zst > listing.txt`

Extract time corresponds to `arx extract archive.arx out_dir` or
`tar --extract -f archive.tar.zst -C out_dir`

Dump time is the time to dump a third of the files in listing.txt (1 every 3 files).
File is extracted using `arx dump archive.arx ${file_path} > dump_file` or
`tar --extract -f archive.tar.zstd ${file_path} > dump_file`
Each command is run `N/3` time in a loop.

Mount diff time is the time to diff the mounted archive with the source directory
```
arx mount archive.arx mount_point &
time diff -r mount_point/linux-5.19 linux-5.19
umount mount_point
```
Mounting the tar archive is made with `archivemount` tool.

The kernel compilation is the time needed to compile the whole kernel with the
default configuration (-j8). For arx, we are compiling the kernel using the
source in the archive mounted in mount_point.

Source directory is stored on a sdd. All test are run on a tmpfs (archive and
extracted files are stored in memory). Kernel compilation is made is "real"
condition. Source or arx archive are stored on ssd.


|                            |  Size   | Creation | Extract | Listing |  Dump  | Dump /entry | Mount diff | Compilation |
| -------------------------- | ------- | -------- | ------- | ------- | ------ | ----------- | ---------- | ----------- |
| linux-5.19_doc             | 58 MB   |          |         |         |        |             | 66ms       |             |
| linux-5.19_doc.tar.ztd     | 7.8 MB  | 8s3      | 68ms    | 51ms    | 2m9s   | 43ms        | 2m38s      |             |
| linux-5.19_doc.arx         | 8.3 MB  | 8s7      | 100ms   | 5ms     | 8s9    | 3.4ms       | 324ms      |             |
| Ratio                      | 1.06    | 1.05     | 1.47    | 0.1     | 0.07   |             | 0.002      |             |
| linux-5.19_drivers         | 865 MB  |          |         |         |        |             | 490ms      |             |
| linux-5.19_drivers.tar.zst | 73 MB   | 1m7      | 688ms   | 570ms   | 1h36   | 520ms       | 2h41m      |             |
| linux-5.19_drivers.arx     | 80 MB   | 3m25     | 930ms   | 19ms    | 35s    | 3.1ms       | 1s75       |             |
| Ratio                      | 1.09    | 3        | 1.35    | 0.03    | 0.006  |             | 0.00018    |             |
| linux-5.19                 | 1326 MB |          |         |         |        |             | 880ms      | 32m         |
| linux-5.19.tar.ztd         | 129 MB  | 1m37s    | 1s130ms | 900ms   | 5h20m  | 833ms       |            |             |
| linux-5.19.arx             | 140 MB  | 4m45s    | 1s47ms  | 45ms    | 1m28s  | 4ms         | 4s2        | 48m         |
| Ratio                      | 1.08    | 2.93     | 1.3     | 0.05    | 0.0045 |             |            |             |


Arx archive are a bit bigger (less than 10%) than tar.zst archive.
Creation time is much longer for arx archive (but a lot of optimisation can be done).
Full extraction time is a bit longer for arx but times are comparable.

Listing files ar accessing individual files from the archive is far more rapid using arx.
Access time is almost constant indpendently of the size of the archive.
For tar however, time to access individual file is greatly increasing when the
archive size is increasing.

Mounting a arx archive make the archive usable without extracting it.
A simple `diff -r` takes 4 more time than a plain diff between two directory but
it is a particular use case (access all files "sequentially" and only once).
But for linux documentation arx is 444 time quicker than tar.
The bigger the tar archive is the bigger is this ratio. I haven't try to do a
mount-diff for the full kernel.

For kernel compilation, the overhead is about 50%. But compilation is made
with `-j8` and arx mount is single threaded (for now) and so all ios are
blocking compilation processes.
But on the opposite side, you can compile the kernel without storing 1.3GB of
source on your hard drive.
