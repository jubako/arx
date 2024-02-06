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
cargo install arx
```


Create an archive
-----------------

Creating an archive is simple :


```
arx create --file my_archive.arx -r my_directory
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

The following compare the performance of Arx to different archive formats.

- Arx, Tar, Squasfs is compressed the content using zstd, level 5.
- Zip is compressed using level 9
- Fs is FileSystem (no archive). Archive creation and extraction is symulated with `cp -a`.

Tests has been done on different data sets :
- the whole linux kernel (linux-5.19)
- the drivers directory in linux kernel
- the document directory in the linux kernel

Source directory is stored on a sdd. All test are run on a tmpfs (archive and
extracted files are stored in memory).

Mount diff time is the time to diff the mounted archive with the source directory
```
arx mount archive.arx mount_point &
time diff -r mount_point/linux-5.19 linux-5.19
umount mount_point
```
Mounting the tar archive is made with `archivemount` tool.

Linux doc
---------

Documentation directory only of linux source code:


|     Type     |  Creation  |   Size   |  Extract   |  Listing  |   Dump   | Mount diff |
| ------------ | ---------- | -------- | ---------- | --------- | -------- | ---------- |
|     Squashfs | 104ms998μs | 10.60 MB |  76ms336μs |  4ms539μs | 11s391ms | 266ms872μs |
| SquashfsFuse |  96ms312μs | 10.60 MB |  70ms490μs |   5ms91μs | 11s601ms | 751ms143μs |
|          Zip |     1s48ms | 15.22 MB | 316ms321μs | 24ms842μs | 40s381ms |      2m37s |
|          Tar | 129ms546μs |  9.68 MB |  78ms268μs | 47ms675μs |       2m |      2m45s |
|          Arx | 204ms192μs | 10.81 MB |  91ms594μs |  6ms308μs | 12s763ms | 234ms275μs |
|           FS | 145ms189μs | 38.45 MB |  90ms135μs |  7ms222μs |  1s942ms |   75ms76μs |

This is the ratio <Archive> time / Arx time.
A ration greater than 100% means Arx is better.


|     Type     | Creation |  Size   | Extract | Listing |  Dump   | Mount diff |
| ------------ | -------- | ------- | ------- | ------- | ------- | ---------- |
|     Squashfs |   51.42% |  98.05% |  83.34% |  71.96% |  89.25% |    113.91% |
| SquashfsFuse |   47.17% |  98.05% |  76.96% |  80.71% |  90.90% |    320.62% |
|          Zip |  513.69% | 140.80% | 345.35% | 393.82% | 316.38% |  67024.17% |
|          Tar |   63.44% |  89.53% |  85.45% | 755.79% | 941.81% |  70433.65% |
|           FS |   71.10% | 355.58% |  98.41% | 114.49% |  15.22% |     32.05% |


Linux Driver
------------

Driver directory only of linux source code:

|     Type     |  Creation  |   Size    |  Extract   |  Listing   |   Dump   | Mount diff |
| ------------ | ---------- | --------- | ---------- | ---------- | -------- | ---------- |
|     Squashfs | 924ms395μs | 121.70 MB | 442ms260μs |  12ms393μs |  43s62ms |    1s451ms |
| SquashfsFuse | 829ms730μs | 121.70 MB | 414ms946μs |  10ms915μs | 42s991ms |    3s664ms |
|          Zip |   20s149ms | 141.91 MB |    3s415ms |  86ms696μs |        - |     47m54s |
|          Tar |    1s267ms |  97.96 MB | 621ms158μs | 508ms895μs |        - |       3h3m |
|          Arx |     2s77ms |  98.23 MB | 792ms979μs |  15ms557μs | 50s516ms |    1s443ms |
|           FS | 771ms946μs | 799.02 MB | 508ms715μs |  18ms608μs |  7s438ms | 488ms789μs |


This is the ratio <Archive> time / Arx time.
A ration greater than 100% means Arx is better.

|     Type     | Creation |  Size   | Extract | Listing  |  Dump  | Mount diff |
| ------------ | -------- | ------- | ------- | -------- | ------ | ---------- |
|     Squashfs |   44.49% | 123.88% |  55.77% |   79.66% | 85.25% |    100.53% |
| SquashfsFuse |   39.94% | 123.88% |  52.33% |   70.16% | 85.10% |    253.76% |
|          Zip |  969.85% | 144.47% | 430.68% |  557.28% |        | 199061.64% |
|          Tar |   61.03% |  99.72% |  78.33% | 3271.16% |        | 762322.34% |
|           FS |   37.16% | 813.39% |  64.15% |  119.61% | 14.72% |     33.85% |



Linux Source Code
-----------------

|     Type     | Creation |   Size    |  Extract   |  Listing   |   Dump   | Mount diff |
| ------------ | -------- | --------- | ---------- | ---------- | -------- | ---------- |
|     Squashfs |  1s493ms | 201.43 MB | 690ms218μs |  22ms892μs |    1m55s |    2s853ms |
| SquashfsFuse |  1s502ms | 201.43 MB | 652ms899μs |  22ms902μs |    1m54s |   13s217ms |
|          Zip | 31s924ms | 252.96 MB |    6s405ms | 226ms491μs |        - |          - |
|          Tar |  2s365ms | 168.77 MB |    1s248ms | 984ms900μs |        - |          - |
|          Arx |  3s757ms | 170.68 MB |    1s297ms |  37ms608μs |    2m10s |    2s911ms |
|           FS |  1s654ms |   1.12 GB | 979ms445μs |  42ms385μs | 17s731ms | 900ms246μs |


This is the ratio <Archive> time / Arx time.
A ration greater than 100% means Arx is better.

|     Type     | Creation |  Size   | Extract | Listing  |  Dump  | Mount diff |
| ------------ | -------- | ------- | ------- | -------- | ------ | ---------- |
|     Squashfs |   39.73% | 118.02% |  53.20% |   60.87% | 88.26% |     98.00% |
| SquashfsFuse |   39.98% | 118.02% |  50.32% |   60.90% | 87.29% |    453.96% |
|          Zip |  849.58% | 148.21% | 493.72% |  602.24% |      - |          - |
|          Tar |   62.94% |  98.88% |  96.20% | 2618.86% |      - |          - |
|           FS |   44.03% | 674.89% |  75.49% |  112.70% | 13.54% |     30.92% |


The kernel compilation is the time needed to compile the whole kernel with the
default configuration (-j8). For arx, we are compiling the kernel using the
source in the archive mounted in mount_point.

Kernel compilation is made is "real" condition. Source or arx archive are stored on ssd.


| Type | Compilation |
| ---- | ----------- |
|  Arx |         40m |
|   FS |         32m |


Arx archive are a bit bigger (about 1%) than tar.zst archive but 15% smaller that squashfr.
Creation and full extraction time are a bit longer for arx but times are comparable.

Listing files ar accessing individual files from the archive is far more rapid using arx or squash.
Access time is almost constant indpendently of the size of the archive.
For tar however, time to access individual file is greatly increasing when the
archive size is increasing.

Mounting a arx archive make the archive usable without extracting it.
A simple `diff -r` takes 4 more time than a plain diff between two directories but
it is a particular use case (access all files "sequentially" and only once).

But for linux documentation arx is 444 time quicker than tar (several hours).
The bigger the tar archive is the bigger is this ratio. I haven't try to do a
mount-diff for the full kernel.

For kernel compilation, the overhead is about 25%.
But on the opposite side, you can compile the kernel without storing 1.3GB of
source on your hard drive.
