"""seine_book liveness: every ```python block must RUN (chapter-local
namespace, blocks executed in reading order); a block marked
`# raises CompileError` must raise exactly that. The book's examples are
load-bearing — prose that shows output shows REAL output, so a surface
change that breaks a chapter turns this red instead of silently rotting
the docs. Run via `make lint-book` (needs the .venv bindings build)."""
import re, io, contextlib, sys, glob

PRE = ("from seine_rs import (fact, Event, Rule, Session, run, this_after, "
       "count, sum_, average, min_, max_, window_time, window_length, "
       "compile_rules, CompileError)\nimport seine_rs as s\n")

failed = 0
for path in sorted(glob.glob('seine_book/[0-9]*.md')):
    src = open(path).read()
    blocks = re.findall(r'```python\n(.*?)```', src, re.S)
    ns = {}
    exec(PRE, ns)
    buf = io.StringIO()
    ok = True
    for i, b in enumerate(blocks):
        expect_raise = '# raises CompileError' in b
        try:
            with contextlib.redirect_stdout(buf):
                exec(b, ns)
            if expect_raise:
                print(f'!! {path} block {i}: expected CompileError, none raised')
                ok = False
        except Exception as e:
            if expect_raise and type(e).__name__ == 'CompileError':
                continue
            print(f'!! {path} block {i}: {type(e).__name__}: {str(e)[:160]}')
            ok = False
            break
    status = 'OK ' if ok else 'FAIL'
    print(f'{status} {path} ({len(blocks)} blocks)')
    for line in buf.getvalue().splitlines():
        print('   |', line)
    if not ok:
        failed += 1
sys.exit(1 if failed else 0)
