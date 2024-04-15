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
|          Arx | 191ms521μs | 10.81 MB |  82ms356μs |  3ms244μs | 12s635ms | 257ms541μs |
|           FS | 151ms691μs | 38.45 MB |  86ms122μs |  8ms500μs |  1s965ms |  75ms854μs |
|     Squashfs | 106ms330μs | 10.60 MB |  78ms341μs |  3ms963μs | 12s519ms | 273ms466μs |
| SquashfsFuse |  93ms131μs | 10.60 MB |  64ms212μs |  4ms100μs | 11s867ms |  724ms92μs |
|          Tar | 132ms570μs |  9.68 MB |  65ms358μs | 43ms247μs |    1m58s |      2m44s |
|          Zip |     1s47ms | 15.22 MB | 318ms528μs | 25ms970μs | 41s219ms |      2m37s |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.


|     Type     |  Creation  |   Size   |  Extract   |  Listing  |   Dump   | Mount diff |
| ------------ | ---------- | -------- | ---------- | --------- | -------- | ---------- |
|           FS |     79.20% |  355.58% |    104.57% |   246.76% |   15.56% |     29.45% |
|     Squashfs |     55.52% |   98.05% |     95.12% |   122.16% |   99.09% |    106.18% |
| SquashfsFuse |     48.63% |   98.05% |     77.97% |   126.39% |   93.93% |    281.16% |
|          Tar |     69.22% |   89.53% |     79.36% |  1333.14% |  938.65% |  63823.89% |
|          Zip |    547.15% |  140.80% |    386.77% |   773.64% |  326.23% |  61193.99% |


Linux Driver
------------

Driver directory only of linux source code:

|     Type     |  Creation  |   Size    |  Extract   |  Listing   |   Dump    | Mount diff |
| ------------ | ---------- | --------- | ---------- | ---------- | --------- | ---------- |
|          Arx |    1s759ms |  98.23 MB | 771ms186μs |   7ms849μs |  47s190ms |    1s348ms |
|           FS | 725ms460μs | 799.02 MB | 437ms409μs |  15ms937μs |   6s938ms | 437ms479μs |
|     Squashfs | 862ms771μs | 121.70 MB |  416ms41μs |   9ms565μs |  46s219ms |    1s419ms |
| SquashfsFuse | 858ms708μs | 121.70 MB | 401ms922μs |   9ms691μs |  46s462ms |    3s680ms |
|          Tar |    1s266ms |  97.96 MB | 604ms497μs | 605ms814μs |     1h24m |         3h |
|          Zip |    20s74ms | 141.91 MB |    3s431ms |  88ms115μs |     6m10s |     47m35s |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     |  Creation  |   Size    |  Extract   |  Listing   |   Dump    | Mount diff |
| ------------ | ---------- | --------- | ---------- | ---------- | --------- | ---------- |
|           FS |     41.22% |   813.39% |     56.72% |    203.04% |    14.70% |     32.45% |
|     Squashfs |     49.03% |   123.88% |     53.95% |    121.86% |    97.94% |    105.31% |
| SquashfsFuse |     48.79% |   123.88% |     52.12% |    123.47% |    98.46% |    272.96% |
|          Tar |     71.97% |    99.72% |     78.39% |   7718.36% | 10737.76% | 803710.62% |
|          Zip |   1140.71% |   144.47% |    444.96% |   1122.63% |   786.06% | 211770.34% |



Linux Source Code
-----------------

|     Type     | Creation |   Size    |  Extract   |  Listing   |   Dump   | Mount diff |
| ------------ | -------- | --------- | ---------- | ---------- | -------- | ---------- |
|          Arx |  3s324ms | 170.68 MB |    1s236ms |  12ms992μs |    1m58s |    2s839ms |
|           FS |  1s591ms |   1.12 GB |  992ms53μs |  40ms451μs | 17s997ms | 878ms367μs |
|     Squashfs |  1s481ms | 201.43 MB | 673ms843μs |  23ms137μs |     2m2s |    2s905ms |
| SquashfsFuse |  1s471ms | 201.43 MB | 646ms649μs |  24ms709μs |     2m2s |     13s8ms |
|          Tar |  1s663ms | 168.77 MB |     1s48ms | 739ms526μs |        - |          - |
|          Zip | 31s261ms | 252.96 MB |    5s936ms | 217ms510μs |        - |          - |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation |   Size    |  Extract   |  Listing   |   Dump   | Mount diff |
| ------------ | -------- | --------- | ---------- | ---------- | -------- | ---------- |
|           FS |   47.86% |   674.89% |     80.25% |    311.35% |   15.17% |     30.94% |
|     Squashfs |   44.57% |   118.02% |     54.51% |    178.09% |  103.05% |    102.35% |
| SquashfsFuse |   44.28% |   118.02% |     52.31% |    190.19% |  103.52% |    458.16% |
|          Tar |   50.04% |    98.88% |     84.80% |   5692.16% |        - |          - |
|          Zip |  940.47% |   148.21% |    480.25% |   1674.18% |        - |          - |

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
