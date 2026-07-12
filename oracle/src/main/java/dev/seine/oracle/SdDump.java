package dev.seine.oracle;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.kie.api.KieBase;
import org.kie.api.definition.type.FactType;
import org.kie.api.event.rule.AfterMatchFiredEvent;
import org.kie.api.event.rule.DefaultAgendaEventListener;
import org.kie.api.io.ResourceType;
import org.kie.api.runtime.KieSession;
import org.kie.internal.utils.KieHelper;

import java.lang.reflect.Method;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.IdentityHashMap;
import java.util.Locale;
import java.util.TimeZone;

/**
 * MEMBER-ORDER graft (D-189; RunnerDump pattern, cloned from ExistsDump).
 * After EVERY firing and every action, dumps each beta node's LEFT and
 * RIGHT tuple memories in PHYSICAL ITERATION ORDER, each entry carrying
 * the fact rendering, the FactHandle id, and identityHashCode tags for
 * tuple and object (the hash-texture detector: run the same scenario in
 * N JVM launches and diff the orders). Ground truth for the L-SD
 * within-item member-order layer — replaces inference from firing
 * sequences. Diagnostic only; never part of the gate.
 *
 * D-194 second lens (the TmsDump graft, re-created in-tree): at the
 * same dump points, every WM handle's EqualityKey status
 * (STATED/JUSTIFIED) + BeliefSet (size, staged WorkingMemoryAction,
 * each LogicalDependency's justifier rule/tuple/liveness), the
 * TMS-side equality-key map (belief presence without WM presence),
 * and the session's pending PropagationEntry queue. TMS lines carry
 * NO identity tags so cross-launch diffs are raw. The TMS object is
 * only ever reached through an existing BeliefSet (or the factory
 * AFTER a key was seen) — the instrument never creates the TMS.
 *
 * Usage: java ... dev.seine.oracle.SdDump <scenario.json>
 */
public final class SdDump {

    private static final String PKG = "seine.gen";
    private static final ObjectMapper M = new ObjectMapper();
    private static int fireNo = 0;

    public static void main(String[] args) throws Exception {
        Locale.setDefault(new Locale("en", "US"));
        TimeZone.setDefault(TimeZone.getTimeZone("UTC"));
        JsonNode scenario = M.readTree(Files.readString(Path.of(args[0])));
        String drl = "package " + PKG + ";\n"
                + declareBlocks(scenario.path("types")) + "\n" + scenario.path("drl").asText();
        KieBase kbase = new KieHelper().addContent(drl, ResourceType.DRL).build();
        KieSession session = kbase.newKieSession();

        session.addEventListener(new DefaultAgendaEventListener() {
            @Override
            public void afterMatchFired(AfterMatchFiredEvent event) {
                StringBuilder sb = new StringBuilder("FIRING ").append(fireNo++).append(' ')
                        .append(event.getMatch().getRule().getName()).append("  ");
                for (Object o : event.getMatch().getObjects()) sb.append(short_(o)).append(" | ");
                System.out.println(sb);
                dumpBetas(session);
                dumpTms(session);
            }
        });

        java.util.List<org.kie.api.runtime.rule.FactHandle> inserted =
                new java.util.ArrayList<>();
        for (JsonNode fact : scenario.path("facts")) {
            inserted.add(session.insert(instantiate(kbase, scenario, fact)));
        }
        System.out.println("== PRE-FIRE ==");
        dumpBetas(session);
        dumpTms(session);
        session.fireAllRules(10_000);
        System.out.println("== FIRE-BOUNDARY ==");
        dumpBetas(session);
        dumpTms(session);
        // D-202/I-RD: epoch replay (OracleRunner's action loop, dump
        // after every action + at each epoch's fire boundary). The TMS
        // recon scenarios use insert/update/delete only.
        int epochNo = 0;
        for (JsonNode epoch : scenario.path("epochs")) {
            epochNo++;
            for (JsonNode action : epoch.path("actions")) {
                String op = action.path("op").asText();
                if (op.equals("insert")) {
                    inserted.add(session.insert(instantiate(kbase, scenario, action)));
                } else if (op.equals("update")) {
                    org.kie.api.runtime.rule.FactHandle fh =
                            inserted.get(action.path("target").asInt());
                    Object bean = session.getObject(fh);
                    FactType ft = kbase.getFactType(PKG, bean.getClass().getSimpleName());
                    java.util.List<String> props = new java.util.ArrayList<>();
                    java.util.Iterator<String> it = action.path("fields").fieldNames();
                    while (it.hasNext()) {
                        String fname = it.next();
                        setField(ft, bean, fname, action.path("fields").path(fname),
                                scenario, bean.getClass().getSimpleName());
                        props.add(fname);
                    }
                    session.update(fh, bean, props.toArray(new String[0]));
                } else if (op.equals("delete")) {
                    session.delete(inserted.get(action.path("target").asInt()));
                } else {
                    throw new IllegalArgumentException("SdDump: unsupported epoch op " + op);
                }
                System.out.println("== ACTION " + op + " ==");
                dumpBetas(session);
                dumpTms(session);
            }
            for (JsonNode fact : epoch.path("facts")) {
                inserted.add(session.insert(instantiate(kbase, scenario, fact)));
            }
            session.fireAllRules(10_000);
            System.out.println("== EPOCH " + epochNo + " BOUNDARY ==");
            dumpBetas(session);
            dumpTms(session);
        }
        session.dispose();
    }

    static void setField(FactType ft, Object bean, String fname, JsonNode v,
            JsonNode scenario, String type) throws Exception {
        JsonNode typedef = null;
        for (JsonNode t : scenario.path("types")) {
            if (t.path("name").asText().equals(type)) typedef = t;
        }
        String fty = "";
        for (JsonNode f : typedef.path("fields")) {
            if (f.path("name").asText().equals(fname)) fty = f.path("type").asText();
        }
        switch (fty) {
            case "i64": ft.set(bean, fname, v.asLong()); break;
            case "f64": ft.set(bean, fname, v.asDouble()); break;
            case "bool": ft.set(bean, fname, v.asBoolean()); break;
            default: ft.set(bean, fname, v.asText());
        }
    }

    static void dumpBetas(KieSession session) {
        try {
            Object kbase = call(session, "getKieBase");
            Object rete = call(kbase, "getRete");
            java.util.Map<?, ?> epsm = (java.util.Map<?, ?>) call(rete, "getEntryPointNodes");
            IdentityHashMap<Object, Boolean> seen = new IdentityHashMap<>();
            for (Object ep : epsm.values()) {
                java.util.Map<?, ?> otns = (java.util.Map<?, ?>) call(ep, "getObjectTypeNodes");
                for (Object otn : otns.values()) walk(otn, session, seen);
            }
        } catch (Throwable t) {
            System.out.println("  dump error: " + t);
        }
    }

    static void walk(Object node, Object reteEval, IdentityHashMap<Object, Boolean> seen) throws Exception {
        if (node == null || seen.put(node, true) != null) return;
        String cn = node.getClass().getSimpleName();
        if (cn.equals("JoinNode") || cn.equals("NotNode") || cn.equals("ExistsNode")) {
            dumpBeta(node, reteEval, cn);
        }
        dumpPaths(node, reteEval, seen);
        for (String prop : new String[]{"getObjectSinkPropagator", "getSinkPropagator"}) {
            try {
                Object propag = call(node, prop);
                if (propag == null) continue;
                Object[] sinks = (Object[]) call(propag, "getSinks");
                for (Object s : sinks) walk(s, reteEval, seen);
            } catch (NoSuchMethodException ignore) {
            }
        }
    }

    static void dumpBeta(Object node, Object reteEval, String cn) throws Exception {
        Object bm = call1(reteEval, "getNodeMemory", node,
                Class.forName("org.drools.core.common.MemoryFactory"));
        Object ltm = call(bm, "getLeftTupleMemory");
        Object rtm = call(bm, "getRightTupleMemory");
        StringBuilder sb = new StringBuilder("  ").append(cn).append(' ').append(call(node, "getId"));
        sb.append("\n     ltm: ");
        Object lit = call(ltm, "iterator");
        for (Object lt = call(lit, "next"); lt != null; lt = call(lit, "next")) {
            sb.append(tupleLabel(lt)).append(' ');
        }
        sb.append("\n     rtm: ");
        Object rit = call(rtm, "iterator");
        for (Object rt = call(rit, "next"); rt != null; rt = call(rit, "next")) {
            sb.append(tupleLabel(rt)).append(' ');
            Object blocked = callOrNull(rt, "getBlocked");
            if (blocked != null) {
                sb.append("blocked{");
                for (Object lt = blocked; lt != null; lt = callOrNull(lt, "getBlockedNext")) {
                    sb.append(tupleLabel(lt)).append(' ');
                }
                sb.append("} ");
            }
        }
        // ltm entries' peer chains (per-path copies of shared-segment tuples)
        sb.append("\n     peers: ");
        Object lit2 = call(ltm, "iterator");
        for (Object lt = call(lit2, "next"); lt != null; lt = call(lit2, "next")) {
            Object peer = callOrNull(lt, "getPeer");
            if (peer != null) {
                sb.append(tupleLabel(lt)).append("->");
                for (Object pp = peer; pp != null; pp = callOrNull(pp, "getPeer")) {
                    sb.append(tupleLabel(pp)).append("->");
                }
                sb.append("| ");
            }
        }
        System.out.println(sb);
    }

    static Object callOrNull(Object o, String m) {
        try { return call(o, m); } catch (Exception e) { return null; }
    }

    // ---- the TMS lens (D-194): BeliefSet per firing; no identity tags ----

    static void dumpTms(KieSession session) {
        try {
            java.util.ArrayList<Object> hs = new java.util.ArrayList<>(session.getFactHandles());
            hs.sort((a, b) -> Long.compare(idOf(a), idOf(b)));
            java.util.HashSet<Long> inWm = new java.util.HashSet<>();
            boolean keySeen = false;
            Object anyBs = null;
            StringBuilder wb = new StringBuilder("  TMS wm:");
            for (Object h : hs) {
                Object o = callOrNull(h, "getObject");
                if (o == null || o.getClass().getSimpleName().equals("InitialFactImpl")) continue;
                long id = idOf(h);
                inWm.add(id);
                wb.append(" @").append(id).append('(').append(short_(o)).append(")key=");
                Object key = callOrNull(h, "getEqualityKey");
                if (key == null) { wb.append('-'); continue; }
                keySeen = true;
                wb.append(stTag(key));
                Object bs = callOrNull(key, "getBeliefSet");
                if (bs != null) { anyBs = bs; wb.append(beliefLabel(bs)); }
            }
            System.out.println(wb);

            Object tms = null;
            if (anyBs != null) {
                Object bsys = callOrNull(anyBs, "getBeliefSystem");
                if (bsys != null) tms = callOrNull(bsys, "getTruthMaintenanceSystem");
            }
            if (tms == null && keySeen) {
                Object fac = Class.forName("org.drools.core.common.TruthMaintenanceSystemFactory")
                        .getMethod("get").invoke(null);
                Class<?> iwmep = Class.forName("org.drools.core.common.InternalWorkingMemoryEntryPoint");
                for (Object ep : session.getEntryPoints()) {
                    try { tms = call1(fac, "getOrCreateTruthMaintenanceSystem", ep, iwmep); } catch (Exception ignore) { }
                    if (tms != null) break;
                }
            }
            StringBuilder kb = new StringBuilder("  TMS keys:");
            if (tms == null) kb.append(" -");
            else {
                java.util.ArrayList<Object> keys = new java.util.ArrayList<>(
                        (java.util.Collection<?>) call(tms, "getEqualityKeys"));
                keys.sort((a, b) -> Long.compare(idOf(callOrNull(a, "getFactHandle")),
                                                 idOf(callOrNull(b, "getFactHandle"))));
                for (Object key : keys) {
                    kb.append(' ').append(stTag(key)).append(" fhs[");
                    int cap = 0;
                    for (Object n = callOrNull(key, "getFirst"); n != null && cap++ < 8;
                            n = callOrNull(n, "getNext")) {
                        long nid = idOf(n);
                        kb.append('@').append(nid).append(inWm.contains(nid) ? "+" : "!");
                    }
                    kb.append(']');
                    Object lfh = callOrNull(key, "getLogicalFactHandle");
                    kb.append("lfh=").append(lfh == null ? "-" : "@" + idOf(lfh));
                    Object bs = callOrNull(key, "getBeliefSet");
                    if (bs != null) kb.append(beliefLabel(bs));
                }
            }
            System.out.println(kb);

            StringBuilder pb = new StringBuilder("  TMS pending:");
            boolean any = false;
            Object it = callOrNull(session, "getActionsIterator");
            if (it instanceof java.util.Iterator) {
                java.util.Iterator<?> i = (java.util.Iterator<?>) it;
                while (i.hasNext()) {
                    Object e = i.next();
                    any = true;
                    pb.append(' ').append(e.getClass().getSimpleName());
                    Object h = fieldOrNull(e, "handle");
                    if (h != null) pb.append('@').append(idOf(h));
                    Object fr = callOrNull(e, "isFullyRetract");
                    Object up = callOrNull(e, "isUpdate");
                    if (fr != null || up != null) pb.append("[fr=").append(fr).append(",up=").append(up).append(']');
                }
            }
            if (!any) pb.append(" -");
            System.out.println(pb);
        } catch (Throwable t) {
            System.out.println("  TMS dump error: " + t);
        }
    }

    /** "bs[n=SIZE wma=-|Cls]" + one dep{...} per LogicalDependency node. */
    static String beliefLabel(Object bs) {
        StringBuilder b = new StringBuilder("bs[n=").append(callOrNull(bs, "size"));
        Object wma = callOrNull(bs, "getWorkingMemoryAction");
        b.append(" wma=").append(wma == null ? "-" : wma.getClass().getSimpleName());
        Object neg = callOrNull(bs, "isNegated");
        Object dec = callOrNull(bs, "isDecided");
        Object con = callOrNull(bs, "isConflicting");
        if (Boolean.TRUE.equals(neg) || Boolean.TRUE.equals(con) || Boolean.FALSE.equals(dec))
            b.append(" neg=").append(neg).append(" dec=").append(dec).append(" con=").append(con);
        b.append(']');
        int cap = 0;
        for (Object m = callOrNull(bs, "getFirst"); m != null && cap++ < 16; m = callOrNull(m, "getNext")) {
            Object dep = callOrNull(m, "getObject");
            if (dep == null) break;
            b.append("dep{").append(depLabel(dep)).append('}');
        }
        return b.toString();
    }

    static String depLabel(Object dep) {
        StringBuilder b = new StringBuilder();
        Object j = callOrNull(dep, "getJustifier");
        if (j == null) b.append("justifier=null");
        else {
            Object rule = callOrNull(j, "getRule");
            b.append(rule == null ? "?" : callOrNull(rule, "getName"));
            b.append(" act=").append(callOrNull(j, "isActive"));
            Object q = callOrNull(j, "isQueued");
            if (q != null) b.append(" q=").append(q);
            Object t = callOrNull(j, "getTuple");
            b.append(" tup=").append(t == null ? "-" : stableTuple(t));
        }
        Object jd = callOrNull(dep, "getJustified");
        if (jd != null) {
            Object jid = callOrNull(jd, "getId");
            b.append(" just=").append(jid != null ? "@" + jid : short_(jd));
        }
        return b.toString();
    }

    /** tupleLabel minus the identity tags: (fact@hid,fact@hid) root-first. */
    static String stableTuple(Object tuple) {
        java.util.ArrayList<String> facts = new java.util.ArrayList<>();
        for (Object t = tuple; t != null; t = callOrNull(t, "getParent")) {
            Object fh = callOrNull(t, "getFactHandle");
            if (fh == null) continue;
            Object o = callOrNull(fh, "getObject");
            if (o != null && !o.getClass().getSimpleName().equals("InitialFactImpl"))
                facts.add(short_(o) + "@" + idOf(fh));
        }
        java.util.Collections.reverse(facts);
        return "(" + String.join(",", facts) + ")";
    }

    static long idOf(Object h) {
        Object id = h == null ? null : callOrNull(h, "getId");
        return id instanceof Number ? ((Number) id).longValue() : -1L;
    }

    static String stTag(Object key) {
        Object st = callOrNull(key, "getStatus");
        if (st == null) return "?";
        try {
            Class<?> ek = Class.forName("org.drools.core.common.EqualityKey");
            int v = (Integer) st;
            if (v == ek.getField("STATED").getInt(null)) return "STATED";
            if (v == ek.getField("JUSTIFIED").getInt(null)) return "JUSTIFIED";
            return "?" + v;
        } catch (Exception e) { return String.valueOf(st); }
    }

    static Object fieldOrNull(Object o, String name) {
        for (Class<?> c = o.getClass(); c != null; c = c.getSuperclass()) {
            try {
                java.lang.reflect.Field f = c.getDeclaredField(name);
                f.setAccessible(true);
                return f.get(o);
            } catch (Exception ignore) { }
        }
        return null;
    }

    static void dumpPaths(Object node, Object reteEval, IdentityHashMap<Object, Boolean> seen2) throws Exception {
        String cn = node.getClass().getSimpleName();
        if (cn.equals("RuleTerminalNode")) {
            try {
                Object pm = call1(reteEval, "getNodeMemory", node,
                        Class.forName("org.drools.core.common.MemoryFactory"));
                Object rule = call(node, "getRule");
                StringBuilder sb = new StringBuilder("  PATH ").append(call(rule, "getName"));
                Object segs = callOrNull(pm, "getSegmentMemories");
                if (segs instanceof Object[]) {
                    int si = 0;
                    for (Object seg : (Object[]) segs) {
                        sb.append("\n     seg").append(si++);
                        if (seg == null) { sb.append(": null"); continue; }
                        Object st = callOrNull(seg, "getStagedLeftTuples");
                        if (st == null) { sb.append(": nostage"); continue; }
                        sb.append(" stagedIns[");
                        for (Object s = callOrNull(st, "getInsertFirst"); s != null; s = callOrNull(s, "getStagedNext"))
                            sb.append(tupleLabel(s)).append(' ');
                        sb.append("] del[");
                        for (Object s = callOrNull(st, "getDeleteFirst"); s != null; s = callOrNull(s, "getStagedNext"))
                            sb.append(tupleLabel(s)).append(' ');
                        sb.append("] upd[");
                        for (Object s = callOrNull(st, "getUpdateFirst"); s != null; s = callOrNull(s, "getStagedNext"))
                            sb.append(tupleLabel(s)).append(' ');
                        sb.append(']');
                    }
                }
                System.out.println(sb);
            } catch (Throwable t) {
                System.out.println("  PATH dump error: " + t);
            }
        }
    }

    /** (facts)@handleId#tupleIdentityHash~objIdentityHash */
    static String tupleLabel(Object tuple) throws Exception {
        java.util.ArrayList<String> facts = new java.util.ArrayList<>();
        Object firstObj = null;
        long hid = -1;
        Object t = tuple;
        while (t != null) {
            Object fh = null;
            try { fh = call(t, "getFactHandle"); } catch (Exception ignore) { }
            if (fh != null && call(fh, "getObject") != null) {
                Object o = call(fh, "getObject");
                if (!o.getClass().getSimpleName().equals("InitialFactImpl")) {
                    facts.add(short_(o));
                    if (firstObj == null) {
                        firstObj = o;
                        try { hid = (long) (int) (Integer) call(fh, "getId"); } catch (Exception ignore) { }
                    }
                }
            }
            Object parent = null;
            try { parent = call(t, "getParent"); } catch (Exception ignore) { }
            t = parent;
        }
        java.util.Collections.reverse(facts);
        return "(" + String.join(",", facts) + ")@" + hid
                + "#" + (System.identityHashCode(tuple) % 100000)
                + "~" + (firstObj == null ? 0 : System.identityHashCode(firstObj) % 100000);
    }

    static String short_(Object o) {
        if (o == null) return "null";
        String s = String.valueOf(o);
        return s.length() > 48 ? s.substring(0, 48) : s;
    }

    static Object call(Object o, String m) throws Exception {
        for (Class<?> c = o.getClass(); c != null; c = c.getSuperclass()) {
            for (Method mm : c.getMethods()) {
                if (mm.getName().equals(m) && mm.getParameterCount() == 0) {
                    mm.setAccessible(true);
                    return mm.invoke(o);
                }
            }
        }
        throw new NoSuchMethodException(m + " on " + o.getClass());
    }

    static Object call1(Object o, String m, Object arg, Class<?> ptype) throws Exception {
        for (Method mm : o.getClass().getMethods()) {
            if (mm.getName().equals(m) && mm.getParameterCount() == 1
                    && mm.getParameterTypes()[0].isAssignableFrom(ptype)) {
                mm.setAccessible(true);
                return mm.invoke(o, arg);
            }
        }
        throw new NoSuchMethodException(m);
    }

    static String declareBlocks(JsonNode types) {
        StringBuilder sb = new StringBuilder();
        for (JsonNode t : types) {
            sb.append("declare ").append(t.path("name").asText()).append('\n');
            for (JsonNode f : t.path("fields")) {
                sb.append("    ").append(f.path("name").asText()).append(" : ")
                  .append(javaType(f.path("type").asText())).append(" @key\n");
            }
            sb.append("end\n");
        }
        return sb.toString();
    }

    static String javaType(String t) {
        switch (t) {
            case "i64": return "long";
            case "f64": return "double";
            case "bool": return "boolean";
            default: return "String";
        }
    }

    static Object instantiate(KieBase kbase, JsonNode scenario, JsonNode fact) throws Exception {
        String type = fact.path("type").asText();
        FactType ft = kbase.getFactType(PKG, type);
        Object obj = ft.newInstance();
        JsonNode typedef = null;
        for (JsonNode t : scenario.path("types")) {
            if (t.path("name").asText().equals(type)) typedef = t;
        }
        for (JsonNode f : typedef.path("fields")) {
            String fn = f.path("name").asText();
            JsonNode v = fact.path("fields").path(fn);
            switch (f.path("type").asText()) {
                case "i64": ft.set(obj, fn, v.asLong()); break;
                case "f64": ft.set(obj, fn, v.asDouble()); break;
                case "bool": ft.set(obj, fn, v.asBoolean()); break;
                default: ft.set(obj, fn, v.asText());
            }
        }
        return obj;
    }
}
