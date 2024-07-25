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

Install arx
===========

Binaries for Windows, MacOS and Linux are available for [every release](https://github.com/jubako/arx/releases).
You can also install arx using Cargo:

```
cargo install arx
```

Use arx
=======

Create an archive
-----------------

Creating an archive is simple :


```
arx create -o my_archive.arx -r my_directory
```

It will one file : `my_archive.arx`, which will contains the `my_directory` directory.


Extract an archive
------------------

Extracting (decompressing) an archive is done with :

```
arx extract my_archive.arx -C my_out_dir
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
# or
arx dump my_archive.arx my_directory/path/to/my_file -o my_file
```

Mounting the archive
--------------------

On linux, you can mount the archive using fuse.

```
mkdir mount_point
arx mount my_archive.arx mount_point
```

If you don't provide a `mount_point`, arx will create a temporary one for you

```
arx mount my_archive.arx # Will create my_archive.arx.xxxxxx
```

`arx` will be running until you unmount `mount_point`.

Converting a zip archive to an arx archive
------------------------------------------

```
zip2arx -o my_archive.arx my_zip_archive.zip
```

Converting a tar archive to an arx archive
------------------------------------------

```
tar2arx -o my_archive.arx my_tar_archive.tar.gz
```

or

```
tar2arx -o my_archive.arx https://example.com/my_tar_archive.tar.gz
```


Performance
===========

The following compare the performance of Arx to different archive formats.

- Arx, Tar, Squasfs is compressed the content using zstd, level 5.
- Zip is compressed using level 9
- Fs is FileSystem (no archive). Archive creation and extraction is simulated with `cp -a`.

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
Mounting the tar and zip archive is made with `archivemount` tool.
Squashfs is mounted using kernel. SquashfsFuse is mounted using fuse API.
Arx mount is implemented using fuse API.

Linux doc
---------

Documentation directory only of linux source code:

|     Type     |  Creation  |   Size   |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | ---------- | -------- | ---------- | ---------- | ---------- | ---------- |
|          Arx | 150ms963μs | 11.10 MB | 038ms395μs | 004ms051μs | 299ms764μs | 005ms618μs |
|           FS | 150ms639μs | 38.45 MB | 106ms821μs | 006ms962μs | 077ms414μs |      498μs |
|     Squashfs | 103ms076μs | 10.60 MB | 098ms787μs | 005ms365μs | 261ms533μs | 002ms088μs |
| SquashfsFuse | 097ms863μs | 10.60 MB |          - |          - | 748ms597μs |          - |
|          Tar | 141ms079μs |  9.68 MB | 065ms744μs | 041ms015μs |     02m41s | 042ms143μs |
|          Zip |   01s083ms | 15.22 MB | 388ms720μs | 037ms044μs |     03m06s | 014ms088μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ---- |
|           FS |     100% | 346% |    278% |    172% |        26% |   9% |
|     Squashfs |      68% |  95% |    257% |    132% |        87% |  37% |
| SquashfsFuse |      65% |  95% |       - |       - |       250% |    - |
|          Tar |      93% |  87% |    171% |   1012% |     53997% | 750% |
|          Zip |     718% | 137% |   1012% |    914% |     62350% | 251% |


Linux Driver
------------

Driver directory only of linux source code:

|     Type     |  Creation  |   Size    |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | ---------- | --------- | ---------- | ---------- | ---------- | ---------- |
|          Arx |   01s060ms |  98.23 MB | 241ms699μs | 009ms516μs |   01s290ms | 007ms193μs |
|           FS | 778ms095μs | 799.02 MB | 523ms191μs | 021ms578μs | 467ms559μs |      495μs |
|     Squashfs | 829ms886μs | 121.70 MB | 435ms851μs | 012ms289μs |   01s629ms | 002ms190μs |
| SquashfsFuse | 829ms237μs | 121.70 MB |          - |          - |   03s823ms |          - |
|          Tar | 911ms042μs |  97.96 MB | 515ms178μs | 472ms060μs |          - | 504ms231μs |
|          Zip |   20s498ms | 141.91 MB |   03s665ms | 098ms194μs |          - | 034ms481μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump  |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ----- |
|           FS |      73% | 813% |    216% |    227% |        36% |    7% |
|     Squashfs |      78% | 124% |    180% |    129% |       126% |   30% |
| SquashfsFuse |      78% | 124% |       - |       - |       296% |     - |
|          Tar |      86% | 100% |    213% |   4961% |          - | 7010% |
|          Zip |    1932% | 144% |   1516% |   1032% |          - |  479% |


Linux Source Code
-----------------

|     Type     | Creation |   Size    |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | -------- | --------- | ---------- | ---------- | ---------- | ---------- |
|          Arx | 02s104ms | 170.97 MB | 435ms846μs | 022ms238μs |   02s829ms | 010ms613μs |
|           FS | 01s605ms |   1.12 GB |   01s046ms | 043ms358μs | 943ms546μs |      493μs |
|     Squashfs | 01s430ms | 201.43 MB | 725ms532μs | 024ms050μs |   03s272ms | 002ms374μs |
| SquashfsFuse | 01s417ms | 201.43 MB |          - |          - |   13s864ms |          - |
|          Tar | 01s479ms | 168.77 MB | 938ms758μs | 799ms550μs |          - | 802ms427μs |
|          Zip | 31s810ms | 252.96 MB |   06s260ms | 256ms137μs |          - | 045ms722μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump  |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ----- |
|           FS |      76% | 674% |    240% |    195% |        33% |    5% |
|     Squashfs |      68% | 118% |    166% |    108% |       116% |   22% |
| SquashfsFuse |      67% | 118% |       - |       - |       490% |     - |
|          Tar |      70% |  99% |    215% |   3595% |          - | 7561% |
|          Zip |    1511% | 148% |   1436% |   1152% |          - |  431% |

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
