SHELL := /bin/bash
# The corpus is TIERED; every tier runs through the same harness, oracle
# (Drools 9.44.0.Final) and canonical comparison:
#   baseline    scenarios adapted from Drools' own regression suite
#               (third-party spec tests; docs/baseline-extraction.md)
#   probes      curated oracle pins behind DECISIONS.md D-0xx entries
#               (probes/, phase0-2 seed suites, demo)
#   regressions fuzzer-found cases, minimized and graduated
# xfail/ holds DOCUMENTED-OPEN divergences (D-042): excluded from the gate,
# consulted by fuzz to suppress re-flagging. baseline-quarantine/ holds
# baseline members under faithfulness-bug triage (report first, then fix).
BASELINE    := $(shell find scenarios/baseline -name '*.json' 2>/dev/null | sort)
PROBES      := $(shell find scenarios/probes scenarios/phase0 scenarios/phase1 scenarios/phase2 scenarios/demo scenarios/failures -name '*.json' 2>/dev/null | sort)
REGRESSIONS := $(shell find scenarios/regressions -name '*.json' 2>/dev/null | sort)

.PHONY: diff diff-baseline diff-probes diff-regressions test oracle fuzz all

all: test diff

# Differential run, reported per tier; fails if any tier fails.
diff: oracle
	@rc=0; \
	echo "=== tier: baseline ($(words $(BASELINE)) scenarios) ==="; \
	cargo run -q -p seine-harness -- diff $(BASELINE) || rc=1; \
	echo "=== tier: probes ($(words $(PROBES)) scenarios) ==="; \
	cargo run -q -p seine-harness -- diff $(PROBES) || rc=1; \
	echo "=== tier: regressions ($(words $(REGRESSIONS)) scenarios) ==="; \
	cargo run -q -p seine-harness -- diff $(REGRESSIONS) || rc=1; \
	exit $$rc

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
