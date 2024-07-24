#!/usr/bin/env python

import subprocess
from time import perf_counter_ns, sleep
from datetime import timedelta
from tempfile import TemporaryDirectory
from pprint import pprint
import argparse, os
from shutil import rmtree

from pathlib import Path
from typing import Any


class Value:
    def __init__(self, v):
        self.v = v

    def __floordiv__(self, other):
        if isinstance(other, int):
            if self.v:
                return type(self)(self.v // other)
            return type(self)(self.v)

    def __truediv__(self, other):
        if self.v is None or other.v is None:
            return Ratio(None)
        return Ratio(self.v / other.v)

    def __iadd__(self, other):
        if self.v is None:
            self.v = other.v
        elif other is not None and other.v is not None:
            self.v += other.v
        return self

    def __str__(self):
        return f"{self.class_name} : {self.display(True)}"


class Ratio:
    def __init__(self, v):
        self.v = v

    def display(self, smart_unit):
        if self.v is None:
            return "-" if smart_unit else ""
        return f"{self.v:.0%}"


class Size(Value):
    class_name = "Size"

    def display(self, smart_unit):
        if not smart_unit:
            return str(self.v)

        B = self.v
        TB, B = divmod(B, 1024 * 1024 * 1024 * 1024)
        GB, B = divmod(B, 1024 * 1024 * 1024)
        MB, B = divmod(B, 1024 * 1024)
        KB, B = divmod(B, 1024)
        if TB:
            size = TB + GB / 1024
            unit = "TB"
        elif GB:
            size = GB + MB / 1024
            unit = "GB"
        elif MB:
            size = MB + KB / 1024
            unit = "MB"
        elif KB:
            size = KB + B / 1024
            unit = "KB"
        else:
            size = B
            unit = "B"

        return f"{size:.2f} {unit}"


class DeltaTime(Value):
    class_name = "Time"
    MILLI_SECOND = 1_000
    SECOND = 1_000_000
    MINUTE = 60 * 1_000_000
    HOUR = 60 * 60 * 1_000_000

    def display(self, smart_unit):
        if not smart_unit:
            return "" if self.v is None else str(self.v / 1_000_000)
        if self.v is None:
            return "-"

        microseconds = self.v
        hours, microseconds = divmod(microseconds, self.HOUR)
        minutes, microseconds = divmod(microseconds, self.MINUTE)
        seconds, microseconds = divmod(microseconds, self.SECOND)
        milliseconds, microseconds = divmod(microseconds, self.MILLI_SECOND)
        if hours:
            # If we have hours, don't care about seconds and under
            seconds = milliseconds = microseconds = 0
        elif minutes:
            # If we have minutes (and not hours), don't care about millisecound and under
            milliseconds = microseconds = 0
        elif seconds:
            # If we are in seconds, don't care about microseconds
            microseconds = 0
        hours = f"{hours}h" if hours else ""
        minutes = f"{minutes:02}m" if minutes else ""
        seconds = f"{seconds:02}s" if seconds else ""
        milliseconds = f"{milliseconds:03}ms" if milliseconds else ""
        microseconds = f"{microseconds:03}μs" if microseconds else ""
        return hours + minutes + seconds + milliseconds + microseconds


def tabular_print(content: list[list[str]]):
    column_sizes = [len(e) for e in content[0]]
    for row in content[1:]:
        for i, c in enumerate(row):
            column_sizes[i] = max(column_sizes[i], len(c))

    def print_row(row, align=">"):
        print("|", end="")
        for size, elem in zip(column_sizes, row):
            print(f" {elem:{align}{size}} |", end="")
        print()

    print_row(content[0], align="^")
    print_row(["-" * s for s in column_sizes])

    for row in content[1:]:
        print_row(row)


def display(v, smart_unit):
    try:
        return v.display(smart_unit)
    except AttributeError:
        return str(v)


def value(v):
    if isinstance(v, str):
        return v
    return v.v or ""


def print_info(infos, smart_unit):
    print("\n")
    columns = list(infos[0].keys())
    rows = [columns]
    for info in infos:
        new_row = [display(info[k], smart_unit) for k in columns]
        rows.append(new_row)
    tabular_print(rows)


def save_csv(infos, csv_path):
    import csv

    columns = list(infos[0].keys())
    with open(csv_path, "w", newline="") as csvfile:
        writer = csv.DictWriter(csvfile, fieldnames=columns, dialect="unix")
        writer.writeheader()
        for info in infos:
            writer.writerow({k: value(v) for k, v in info.items()})


def stringify(l: list[Any]) -> list[str]:
    return [str(e) for e in l]


class SkipCommand(Exception):
    pass


have_sudo_run = False


def run(commands: list[list[str]], *, stdout=None, wait=True, verbose) -> int:
    global have_sudo_run
    if verbose:
        print(commands)
    if not commands:
        return DeltaTime(0)
    elapsed_time = None
    start_time = perf_counter_ns()
    if len(commands) == 1:
        command = commands[0]
        is_first_sudo = command[0] == "sudo" and not have_sudo_run
        if is_first_sudo:
            print("!!!!!!!!!! We will run the sudo command:")
        if verbose or is_first_sudo:
            print(stringify(command))
        process = subprocess.Popen(stringify(commands[0]), stdout=stdout)
        if is_first_sudo:
            sleep(10)
            have_sudo_run = True
    else:
        first_command, *middle_commands, last_command = commands
        process = subprocess.Popen(stringify(first_command), stdout=subprocess.PIPE)
        for c in middle_commands:
            new_process = subprocess.Popen(
                stringify(c), stdin=process.stdout, stdout=subprocess.PIPE
            )
            process.stdout.close()
            process = new_process
        new_process = subprocess.Popen(
            stringify(last_command), stdin=process.stdout, stdout=stdout
        )
        process.stdout.close()
        process = new_process
    if wait:
        process.wait()
        elapsed_time = perf_counter_ns() - start_time
        if process.returncode != 0:
            raise subprocess.CalledProcessError(process.returncode, commands)
        return DeltaTime(elapsed_time // 1000)
    return process


KNOWN_KINDS = {}


def register(klass):
    KNOWN_KINDS[klass.name] = klass
    return klass


class ArchiveKind:
    mount_too_long = False

    @staticmethod
    def size(archive):
        return Size(archive.stat().st_size)

    @staticmethod
    def unmount(mount_dir):
        return [["umount", mount_dir]]


@register
class Raw(ArchiveKind):
    name = "FS"
    extension = "dir"

    @staticmethod
    def creation(source, archive):
        archive.mkdir()
        return [["cp", "-a", source, "-t", archive]]

    @staticmethod
    def size(archive):
        archive_size = 0
        for dir_path, dirnames, filenames in archive.walk():
            for n in filenames:
                archive_size += (dir_path / n).stat().st_size
        return Size(archive_size)

    @staticmethod
    def list(archive):
        return [["find", archive]]

    @staticmethod
    def extract(archive, out_dir):
        return [["cp", "-a", *archive.glob("*"), "-t", out_dir]]

    @staticmethod
    def mount(archive, mount_dir):
        return [["ln", "-s", *archive.glob("*"), "-t", mount_dir]]

    @staticmethod
    def dump(archive, file):
        return [["cat", archive / file]]

    @staticmethod
    def unmount(mount_dir):
        return []


@register
class Arx(ArchiveKind):
    name = "Arx"
    extension = "arx"

    @staticmethod
    def creation(source, archive) -> list[str]:
        return [["arx", "create", source, "-r", "-o", archive]]

    @staticmethod
    def list(archive) -> list[str]:
        return [["arx", "list", archive]]

    @staticmethod
    def extract(archive, out_dir):
        return [["arx", "extract", archive, "-C", out_dir]]

    @staticmethod
    def mount(archive, mount_dir):
        return [["arx", "mount", archive, mount_dir]]

    @staticmethod
    def dump(archive, file):
        return [["arx", "dump", archive, file]]


@register
class Tar(ArchiveKind):
    name = "Tar"
    extension = "tar.zst"
    mount_too_long = True

    @staticmethod
    def creation(source, archive):
        return [
            ["tar", "-c", source],
            ["zstd", "--no-progress", "-q", "-5", "-T16", "-o", archive],
        ]

    @staticmethod
    def list(archive):
        return [["tar", "--list", "-f", archive]]

    @staticmethod
    def extract(archive, out_dir):
        return [["tar", "--extract", "-f", archive, "-C", out_dir]]

    @staticmethod
    def mount(archive, mount_dir):
        return [["archivemount", archive, mount_dir]]

    @staticmethod
    def dump(archive, file):
        return [["tar", "--extract", "-O", "-f", archive, file]]


@register
class Zip(ArchiveKind):
    name = "Zip"
    extension = "zip"
    mount_too_long = True

    @staticmethod
    def creation(source, archive):
        return [["zip", "-9qr", archive, source]]

    @staticmethod
    def list(archive):
        return [["unzip", "-l", archive]]

    @staticmethod
    def extract(archive, out_dir):
        return [["unzip", "-q", archive, "-d", out_dir]]

    @staticmethod
    def mount(archive, mount_dir):
        return [["archivemount", archive, mount_dir]]

    @staticmethod
    def dump(archive, file):
        return [["unzip", "-p", archive, file]]


class Squashfs(ArchiveKind):
    extension = "sqsh"

    @staticmethod
    def creation(source, archive):
        return [
            [
                "mksquashfs",
                source,
                archive,
                "-quiet",
                "-no-progress",
                "-no-xattrs",
                "-no-strip",
                "-comp",
                "zstd",
                "-keep-as-directory",
                "-Xcompression-level",
                "5",
            ]
        ]

    @staticmethod
    def list(archive):
        return [["unsquashfs", "-q", "-l", archive]]

    @staticmethod
    def extract(archive, out_dir):
        return [["unsquashfs", "-q", "-no-progress", "-d", out_dir, archive]]

    @staticmethod
    def dump(archive, file):
        return [["unsquashfs", "-q", "-cat", "-p", "1", archive, file]]


@register
class SquashfsKernel(Squashfs):
    name = "Squashfs"

    @staticmethod
    def mount(archive, mount_dir):
        command = ["sudo", "mount", archive, mount_dir, "-o", "loop"]
        return [command]

    @staticmethod
    def unmount(mount_dir):
        return [["sudo", "umount", mount_dir]]


@register
class SquashfsFuse(Squashfs):
    name = "SquashfsFuse"

    @staticmethod
    def list(archive):
        raise SkipCommand

    @staticmethod
    def extract(archive, out_dir):
        raise SkipCommand

    @staticmethod
    def dump(archive, file):
        raise SkipCommand

    @staticmethod
    def mount(archive, mount_dir):
        command = ["squashfuse", archive, mount_dir]
        return [command]


class Comparator:
    def __init__(
        self,
        ref_file_list: list[str],
        tmp_dir: Path,
        kind,
        run,
        args,
    ):
        self.ref_file_list = ref_file_list
        self.tmp_dir = tmp_dir
        self.archive = tmp_dir / f"archive.{kind.extension}"
        self.info = {"Type": kind.name}
        self.kind = kind
        self.source = args.source
        self.verbose = args.verbose >= 1
        self.trace = args.verbose >= 2
        self.debug = args.verbose >= 3
        self.fast = args.fast
        self.run = run
        self.too_long_operations = args.too_long_operations

    def creation(self):
        return run(self.kind.creation(self.source, self.archive), verbose=self.debug)

    def size(self):
        return self.kind.size(self.archive)

    def listing(self):
        file_list = self.tmp_dir / "file_list.txt"
        with file_list.open("w") as f:
            return run(self.kind.list(self.archive), stdout=f, verbose=self.debug)

    def extract(self):
        out_dir = self.tmp_dir / "OUT_DIR"
        out_dir.mkdir()
        return run(self.kind.extract(self.archive, out_dir), verbose=self.debug)

    def dump(self):
        dump_time = DeltaTime(0)
        to_dump = (f for i, f in enumerate(self.ref_file_list) if i % 10 == 0)
        len_to_dump = len(self.ref_file_list) // 10
        dump_dir = self.tmp_dir / "DUMP_DIR"
        for i, file in enumerate(to_dump):
            if self.trace and i % 100 == 0:
                print(f"\r{i}/{len_to_dump}", end="")
            out_file = dump_dir / file
            out_file.parent.mkdir(parents=True, exist_ok=True)
            with out_file.open("w") as f:
                dump_time += run(
                    self.kind.dump(self.archive, file),
                    stdout=f,
                    verbose=(self.debug and i == 0),
                )
            if dump_time.v >= 2 * DeltaTime.MINUTE:
                break
        dump_time = dump_time // (i + 1)
        if self.trace:
            print(f"\r{i}/{len_to_dump}")
        return dump_time

    def mount_diff(self):
        if self.kind.mount_too_long and not self.too_long_operations:
            raise SkipCommand
        mount_dir = self.tmp_dir / "MOUNT_DIR"
        mount_dir.mkdir()
        mount_process = run(
            self.kind.mount(self.archive, mount_dir), wait=False, verbose=self.debug
        )
        sleep(1)  # Wait for directory being mounted
        try:
            return run(
                [["diff", "-r", self.source, mount_dir / self.source.name]],
                verbose=self.debug,
            )
        finally:
            run(self.kind.unmount(mount_dir), verbose=self.debug)

    def compare(self):
        operations = ["creation", "size", "extract", "listing", "mount_diff"]
        if not self.fast:
            operations.append("dump")
        for operation in operations:
            if self.trace:
                print(f"--- {operation} {self.archive}")
            try:
                time = getattr(self, operation)()
            except SkipCommand:
                if self.trace:
                    print("Skip")
                time = DeltaTime(None)
            cap_operation = operation[0].capitalize() + operation[1:]
            cap_operation = cap_operation.replace("_", " ")
            self.info[cap_operation] = time
            if self.trace:
                print(time)
        return self.info


class MultiComparator:
    def __init__(
        self,
        ref_file_list: list[str],
        tmp_dir: Path,
        kind,
        args,
    ):
        self.ref_file_list = ref_file_list
        self.tmp_dir = tmp_dir
        self.kind = kind
        self.verbose = args.verbose >= 1
        self.runs = args.runs
        self.args = args

    def compare(self):
        infos = None
        for run in range(self.runs):
            if self.verbose:
                print(f"\n====== Testing {self.kind.name}:{run}")
            run_tmp_dir = self.tmp_dir / f"{run}"
            run_tmp_dir.mkdir(parents=True)
            sleep(2)  # Let CPU and such sleep a bit to have a free start
            comparator = Comparator(
                self.ref_file_list, run_tmp_dir, self.kind, run, self.args
            )
            new_infos = comparator.compare()
            if infos is None:
                infos = new_infos
            else:
                for key in infos.keys():
                    if key == "Type":
                        continue
                    infos[key] += new_infos[key]
            rmtree(run_tmp_dir)

        for key in infos.keys():
            if key == "Type":
                continue
            infos[key] = infos[key] // self.runs

        return infos


def main():
    parser = argparse.ArgumentParser(
        prog="ArchiveComparator",
        description="A small tool to compare arx to other archive formats",
    )

    parser.add_argument("source", type=Path)
    parser.add_argument("--bin_dir", type=Path)
    parser.add_argument(
        "--kinds", action="extend", nargs="+", choices=KNOWN_KINDS.keys()
    )
    parser.add_argument("--verbose", "-v", action="count", default=0)
    parser.add_argument("--save-csv", type=Path)
    parser.add_argument("--fast", action="store_true", default=False)
    parser.add_argument("--too-long-operations", action="store_true", default=False)
    parser.add_argument("--runs", type=int, default=1)
    args = parser.parse_args()

    if not args.source.is_dir():
        print(f"{args.source} must be a directory")
        exit(-1)

    ref_file_list = []
    for dir_path, dirnames, filenames in args.source.walk():
        ref_file_list.extend((dir_path / n for n in dirnames))
        ref_file_list.extend((dir_path / n for n in filenames))
    ref_file_list = [f for f in ref_file_list if not f.is_symlink() and f.is_file()]
    ref_file_list.sort()

    if args.bin_dir and args.bin_dir.is_dir():
        print(f"Adding {args.bin_dir} to PATH")
        os.environ["PATH"] = f"{args.bin_dir}:{os.environ['PATH']}"

    infos = []
    with TemporaryDirectory(prefix="tmp_arx_cmp_", delete=True) as tmp_dir:
        tmp_dir = Path(tmp_dir)
        for kind in args.kinds:
            k_tmp_dir = tmp_dir / kind
            k_tmp_dir.mkdir()
            comparator = MultiComparator(
                ref_file_list, k_tmp_dir, KNOWN_KINDS[kind], args
            )
            rmtree(k_tmp_dir)
            infos.append(comparator.compare())

    print("======  Results")
    infos.sort(key=lambda e: e["Type"])

    if args.save_csv:
        save_csv(infos, args.save_csv)
    print_info(infos, True)
    print_info(infos, False)

    ref = [info for info in infos if info["Type"] == "Arx"]
    if ref:
        ref = ref[0]
        infos.remove(ref)
        for info in infos:
            for key in ref.keys():
                if key == "Type":
                    continue
                info[key] = info[key] / ref[key]
        if infos:
            print_info(infos, True)


if __name__ == "__main__":
    main()
