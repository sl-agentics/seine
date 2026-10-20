"""Command line: `python -m seine_rs table [--matrix] [--rules] TABLE.csv` compiles a Drools CSV decision table."""
import sys

from .tables import main as _table_main


def _main(argv):
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__.strip(), file=sys.stderr if not argv else sys.stdout)
        return 2 if not argv else 0
    if argv[0] == "table":
        return _table_main(argv[1:])
    print(f"error: unknown command {argv[0]!r} (commands: table)", file=sys.stderr)
    return 2


raise SystemExit(_main(sys.argv[1:]))
