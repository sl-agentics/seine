SHELL := /bin/bash
# The corpus is TIERED; every tier runs through the same harness, oracle
# (Drools 9.44.0.Final) and canonical comparison:
#   baseline    scenarios adapted from Drools' own regression suite
#               (third-party spec tests; docs/baseline-extraction.md)
#   probes      curated oracle pins behind DECISIONS.md D-0xx entries
#               (probes/, phase0-2 seed suites, demo)
#   regressions fuzzer-found cases, minimized and graduated
# xfail/ holds DOCUMENTED-OPEN divergences (D-042): excluded from the
# ORACLE diff by design (they diverge; that is their finding), but engine
# output on them is gated against a banked snapshot (xfail-drift, D-187) —
# movement must be deliberate (re-triage + xfail-rebank + D-entry), never
# silent (the D-091/D-101 lesson, caught at D-186). fuzz consults xfail/
# to suppress re-flagging. baseline-quarantine/ holds baseline members
# under faithfulness-bug triage (report first, then fix).
BASELINE    := $(shell find scenarios/baseline -name '*.json' 2>/dev/null | sort)
PROBES      := $(shell find scenarios/probes scenarios/phase0 scenarios/phase1 scenarios/phase2 scenarios/demo scenarios/failures -name '*.json' 2>/dev/null | sort)
REGRESSIONS := $(shell find scenarios/regressions -name '*.json' 2>/dev/null | sort)

.PHONY: diff diff-baseline diff-probes diff-regressions test oracle fuzz all xfail-drift xfail-rebank

all: test diff

# Differential run, reported per tier; fails if any tier fails.
# The xfail engine-drift gate rides along (engine-only, no oracle).
diff: oracle
	@rc=0; \
	echo "=== tier: baseline ($(words $(BASELINE)) scenarios) ==="; \
	cargo run -q -p seine-harness -- diff $(BASELINE) || rc=1; \
	echo "=== tier: probes ($(words $(PROBES)) scenarios) ==="; \
	cargo run -q -p seine-harness -- diff $(PROBES) || rc=1; \
	echo "=== tier: regressions ($(words $(REGRESSIONS)) scenarios) ==="; \
	cargo run -q -p seine-harness -- diff $(REGRESSIONS) || rc=1; \
	echo "=== tier: xfail engine-drift (banked snapshot) ==="; \
	python3 tools/xfail_drift.py || rc=1; \
	exit $$rc

# D-187: engine-drift gate over the documented-open quarantine.
xfail-drift:
	python3 tools/xfail_drift.py

# Deliberate movement only: re-triage (tools/triage_xfail.py) + D-entry.
xfail-rebank:
	python3 tools/xfail_drift.py --rebank

# D-097: data-type semantics differential (DuckDB oracle, D-095 axis 3)
diff-duckdb:
	.venv/bin/python tools/diff_duckdb.py scenarios/duckdb/*.json

# D-102: probe liveness + engine-validity + fence-regression audit
lint-probes:
	python3 tools/lint_probes.py

diff-baseline: oracle
	cargo run -q -p seine-harness -- diff $(BASELINE)

diff-probes: oracle
	cargo run -q -p seine-harness -- diff $(PROBES)

diff-regressions: oracle
	cargo run -q -p seine-harness -- diff $(REGRESSIONS)

# Rust unit + characterization tests (no JVM needed).
test:
	cargo test -q

# Differential fuzzing (deterministic; pass SEED=n CASES=n to vary).
# The fuzzer's role is to EXPLORE BEYOND THE BASELINE tier — the spec floor
# comes from the Drools-suite baseline; fuzz hunts what no suite thought of.
SEED ?= 42
CASES ?= 10000
fuzz: oracle
	cargo run -q -p seine-harness -- fuzz $(CASES) $(SEED)

# Build the Java reference runner (Drools pinned in oracle/pom.xml).
oracle: oracle/target/classpath.txt

oracle/target/classpath.txt: oracle/pom.xml $(shell find oracle/src -type f)
	cd oracle && mvn -q -DskipTests package
