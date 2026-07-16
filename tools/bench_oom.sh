#!/bin/bash
# Who-OOMs-first (D-269): engine vs oracle memory endurance under EQUAL
# kernel cgroup caps (systemd-run --user, MemoryMax — total RSS is the
# law for both; -Xmx merely matches the JVM heap to its budget).
# Workload: the disjoint join (N facts per side, ZERO firings) — pure
# fact/alpha/beta-memory pressure with no fire-limit ceiling.
#
# Usage: tools/bench_oom.sh [BUDGET_GB] N [N...]
#   e.g.  tools/bench_oom.sh 16 4000000 4500000 5000000
set -u
BUDGET=${1:?budget GB}; shift
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
cd "$(dirname "$0")/.."
CP="oracle/target/classes:$(cat oracle/target/classpath.txt)"
CG="systemd-run --user --scope --quiet -p MemoryMax=${BUDGET}G -p MemorySwapMax=0"
gen() { python3 - "$1" "$2" << 'PYEOF'
import sys
n = int(sys.argv[1])
with open(sys.argv[2], 'w') as f:
    f.write('{"name":"oom_%d","drl":"rule \\"R0\\"\\nwhen\\n    T0($k : k)\\n    T1(k == $k)\\nthen\\nend\\n",'
            '"types":[{"name":"T0","fields":[{"name":"k","type":"i64"}]},{"name":"T1","fields":[{"name":"k","type":"i64"}]}],'
            '"epochs":[],"facts":[' % n)
    f.write(','.join('{"type":"T0","fields":{"k":%d}}' % i for i in range(n)))
    f.write(',')
    f.write(','.join('{"type":"T1","fields":{"k":%d}}' % (i + 1000000000) for i in range(n)))
    f.write(']}')
PYEOF
}
for N in "$@"; do
    F=$SCRATCH/oom_$N.json
    gen $N $F
    /usr/bin/time -v $CG timeout 600 ./target/release/seine-harness run $F > /dev/null 2> $SCRATCH/e.err
    erc=$?; erss=$(awk -F: '/Maximum resident/{print int($2/1024)}' $SCRATCH/e.err)
    ewall=$(awk '/Elapsed \(wall/{print $NF}' $SCRATCH/e.err)
    case $erc in 0) es=ok;; 124) es=TIMEOUT;; *) es="KILLED";; esac
    /usr/bin/time -v $CG timeout 600 java -Xmx${BUDGET}g -cp "$CP" dev.seine.oracle.OracleRunner $F \
        > $SCRATCH/o.json 2> $SCRATCH/o.err
    orc=$?; orss=$(awk -F: '/Maximum resident/{print int($2/1024)}' $SCRATCH/o.err)
    owall=$(awk '/Elapsed \(wall/{print $NF}' $SCRATCH/o.err)
    if grep -q '"error"' $SCRATCH/o.json 2>/dev/null; then os="OOM-caught"
    elif [ $orc -eq 124 ]; then os=TIMEOUT
    elif [ $orc -ne 0 ]; then os="KILLED"
    else os=ok; fi
    echo "N=$N | engine: $es rss=${erss}MB wall=$ewall | drools: $os rss=${orss}MB wall=$owall"
done
