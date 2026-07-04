SHELL := /bin/bash
SCENARIOS := $(shell find scenarios -name '*.json' | sort)

.PHONY: diff test oracle fuzz all

all: test diff

# Differential run: every scenario through both engines, canonical compare.
diff: oracle
	cargo run -q -p seine-harness -- diff $(SCENARIOS)

# Rust unit + characterization tests (no JVM needed).
test:
	cargo test -q

# Differential fuzzing (deterministic; pass SEED=n CASES=n to vary).
SEED ?= 42
CASES ?= 10000
fuzz: oracle
	cargo run -q -p seine-harness -- fuzz $(CASES) $(SEED)

# Build the Java reference runner (Drools pinned in oracle/pom.xml).
oracle: oracle/target/classpath.txt

oracle/target/classpath.txt: oracle/pom.xml $(shell find oracle/src -type f)
	cd oracle && mvn -q -DskipTests package
