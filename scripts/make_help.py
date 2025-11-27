from pathlib import Path
from argparse import ArgumentParser
from typing import Dict, List, Tuple
from collections import defaultdict

# ANSI color codes
class Colors:
    YELLOW = "\033[33m"
    BLUE = "\033[34m"
    RESET = "\033[0m"

def parse_makefile(file: Path) -> Dict[str, List[Tuple[str, str]]]:
    """Parse a Makefile and extract help information.
    
    Args:
        file: Path to the Makefile
    Returns:
        Dictionary containing grouped help information
    """
    result = defaultdict(list)
    try:
        with open(file, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line.startswith("## "):
                    continue
                
                try:
                    data = line.lstrip("## ").split(":", 2)
                    if len(data) != 3:
                        continue
                        
                    group, command, description = map(str.strip, data)
                    result[group].append((command, description))
                except ValueError as e:
                    print(f"Warning: Skipping malformed line in {file}: {line}")
                    
    except FileNotFoundError:
        print(f"Error: Could not find Makefile: {file}")
    except Exception as e:
        print(f"Error processing {file}: {str(e)}")
        
    return dict(result)

def create_help(files: List[Path]) -> Dict[str, List[Tuple[str, str]]]:
    """Combine help information from multiple Makefiles.
    
    Args:
        files: List of Makefile paths
    Returns:
        Combined help information dictionary
    """
    result = defaultdict(list)
    for file_path in files:
        file_results = parse_makefile(file_path)
        for group, targets in file_results.items():
            result[group].extend(targets)
    return dict(result)

def print_help(help_dict: Dict[str, List[Tuple[str, str]]]) -> None:
    """Print formatted help information.
    
    Args:
        help_dict: Dictionary containing help information
    """
    if not help_dict:
        print("No help information found in the specified Makefiles.")
        return
        
    for group, targets in sorted(help_dict.items()):
        print(f"    {Colors.YELLOW}[{group}]{Colors.RESET}")
        for command, description in sorted(targets):
            print(f"    {command: <32}{Colors.BLUE}# {description}{Colors.RESET}")
        print()

def main() -> None:
    """Main function to process command line arguments and display help."""
    parser = ArgumentParser(description="Generate help information from Makefiles")
    parser.add_argument(
        "files",
        nargs="+",
        type=Path,
        help="Makefiles to generate help from"
    )

    args = parser.parse_args()
    help_dict = create_help(args.files)

    print("A list of available targets and variables")
    print()
    print_help(help_dict)

if __name__ == "__main__":
    main()