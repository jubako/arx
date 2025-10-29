# Arx 0.4.0

- Add an option `--overwrite` to specify how `arx extract` overwrite existing files.
- Better extract api to select which files to extract.
- Add `--allow-other` and `--allow-root` to `arx mount` to allow sharing the mount point
  with other or root users.
- Better create api: Simple trimming of added path instead of `--strip-prefix`
- Automatically detect the current shell when using option `--generate-complete`
- Remove deprecied (and hidden) options.
- Limit number of open fd to 1000 in the same time.
- Add licences file to Cargo.toml
- Test improvements:
 . Use `rustest` to run our test.
 . Don't use `arx_test_dir` to generate testing content
- Adapt to new Jubako api (error types, variant and property names, SmallVec, utf8 locator, array cmp)
- Update README

## Zip2Arx

- Handle Ntfs extra field stored in zip file.

## Tar2Arx

- Handle gnu header in tar file.

# Arx 0.3.2

- `--version` option now includes the git commit.
- `arx create` has a new option `--follow-symlink` telling if arx must follow symlink or
  create the entry as a symlink.
- Fix various small issues when creating a arx giving a file to add through a file list (`-L` option).
- Check input paths given on command line before starting arx creation.
- Add help content to `arx create --help`.
- Add more testing on arx command line.
- Fix 32bits compilation of python wrapping.
- Fix python CI publishing.
- Use `jbk::cmd_utils` instead of our own.
- Update README

# Arx 0.3.1

- Use version 0.3.1 of Jubako
- Do not crash on broken pipe (SIGPIPE)
- `arx mount` and `arx extract` gain `--root-dir` option. If given, the directory (inside arx) will
  be used as root instead of default root.
- Fix Python CI and metadata
- Small fixes (warning, dependencies, ...)

# Arx 0.3.0

This release is based on version 0.3.0 of Jubako.
This is a major release, see Jubako changelog for changes impacting arx.
Main information to remember of Jubako release is that the format as evolved and compatibility
with previous version is broken.

If you have existing archives, you can convert it to new format by mounting it (with a previous version of arx)
and recreate it with a new version.


This changelog is about Arx itself.

- Adapt to various change in the Jubako API
- `tar2arx` accepts now a http(s) url to a tar archive. It will convert the archive as it
  downloads it.
- `tar2arx` now infers the name of the arx archive to create from the name of tar archive.
- `arx mount` not automatically create a temporary mount point is none is given.
- `arx mount` now run in background (NOHUP). Option `--foreground` is added to keep previous behavior.
- `tar2arx` and `zip2arx` packages have now features to configure supported compression algorithmes.
- `arx create` now have an option `--progress` to print progress bar.
- `arx create` better handle input path (must be a relative utf8 path), symlinks
- `arx extract` now have an option `--recurse` to extrat a directory and its content.
- Add a python wrapper onto libarx.
- Performance improvement, mainly parrallalisation of extract operation
- Add the `fuse` feature (in default features). This allow user to compile without fuse.

# Arx 0.2.1

- Add README.md in all sub-packages.
- Improve performance of arx list (x2)
- libarx now creates missing intermediate "directory" at arx archive creation.
- `arx dump` now takes a output argument to not always dump on stdout
- `tar2arx` accepts now a compressed tar as input.
- `tar2arx` can now takes tar as a path to the tar and not always as stdin.
- Add option to generate man page (all tools)
- Add option to generate completion script (all tools)
- `arx create` now takes option `-o` for create archive. (`-f` is keep for compatibility but will be removed)
- `arx extract` now takes the input archive as argument (`-f` is keep for compatibility but will be removed)
- Show a nice message in case of panic.
- Better CI

# Arx 0.2

This release is a huge release!!
Most of the changes come from the Jubako 0.2.0 release.

## Create a arx library

Arx is now split in a library and a tool part.

The arx library (libarx) can now be used by other projects.

## New `automount` binary.

The `automount` binary is useless in itself. But it can be concatenated with an Arx archive to
create an automount archive.


## Store unix properties in Arx

Now arx format stores file properties. The properties are:

- owner
- group
- mode
- mtime

The size is also store in the entry, which allow libarx to give the file size without searching
for content in the contentPack (this somehow prevents use to use variant contentPacks).

## Do deduplication at file level.

At creation, arx maintain a cache of hash of already added content.
If a file to add has the same hash that a previously added content, we will reuse the previous ContentAddress.

## Various improvement of the command line API

- `arx list` now have a stable output,
- `arx extract` and `arx create` can now take a file listing the files to extract/add
- `arx extract` has a `--progress` option
- `arx create` has an option to strip prefix from the add file name
- `arx create` has an option to change current directory before creating the archive
- `arx create` has options to specify if we want one file (containing all packs) or one file per packs
- `arx create` has an option to configure the compression used

## Add zip2arx and tar2arx

## Port to Windows and MacOs

While arx compile on Windows and MacOs, it is not completely tested.
`mount` feature is not available on Windows.

## Various and numerous performance improvements
