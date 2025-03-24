from pathlib import Path
from argparse import ArgumentParser


YELLOW = "\033[33m"
BLUE = "\033[34m"
RESET = "\033[0m"


def parse_makefile(file: Path, res: dict[str, list[tuple[str, str]]]):
    with open(file, "r") as f:
        contents = f.readlines()
        contents = map(lambda x: x.strip(), contents)
        contents = filter(lambda x: x.startswith("## "), contents)
        contents = map(lambda x: x.lstrip("## "), contents)

        for line in contents:
            data = line.split(":")
            assert len(data) == 3

            grp = data[0].strip()
            cmd = data[1].strip()
            val = data[2].strip()

            if grp in res:
                res[grp].append((cmd, val))
            else:
                res[grp] = [(cmd, val)]


def create_help(files: list[Path]) -> dict[str, list[tuple[str, str]]]:
    res = dict()
    for file_path in files:
        parse_makefile(file_path, res)
    return res


def print_help(res: dict[str, list[tuple[str, str]]]):
    for grp, targets in res.items():
        print(f"    {YELLOW}[{grp}]{RESET}")
        for cmd, val in targets:
            print(f"    {cmd: <32}{BLUE}# {val}{RESET}")
        print()


if __name__ == "__main__":
    parser = ArgumentParser()

    parser.add_argument("files", nargs="+", help="Makefiles to generate help from")

    args = parser.parse_args()

    help_dict = create_help(args.files)

    print("A list of available targets and variables")
    print()
    print_help(help_dict)
