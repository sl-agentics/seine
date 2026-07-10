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

import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.IdentityHashMap;
import java.util.Locale;
import java.util.TimeZone;

/**
 * D-090a ground-truth graft (RunnerDump pattern): runs a scenario like
 * OracleRunner but dumps every AccumulateNode's per-left state after
 * each firing — left fact, function-context internals (reflected),
 * match chain (right fact + per-match stored contribution), and the
 * live result object. Diagnostic only; never part of the gate.
 */
public final class AccDump {

    private static final String PKG = "seine.gen";
    private static final ObjectMapper M = new ObjectMapper();

    public static void main(String[] args) throws Exception {
        Locale.setDefault(new Locale("en", "US"));
        TimeZone.setDefault(TimeZone.getTimeZone("UTC"));
        JsonNode scenario = M.readTree(Files.readString(Path.of(args[0])));
        String drl = "package " + PKG + ";\n"
                + "import java.util.List;\nimport java.util.ArrayList;\nimport java.util.Collection;\n"
                + declareBlocks(scenario.path("types")) + "\n" + scenario.path("drl").asText();
        // Must mirror OracleRunner's session construction EXACTLY: event
        // scenarios need STREAM mode + a pseudo clock, else Drools runs in
        // CLOUD mode and the temporal-join firing ORDER differs from the gate
        // (CLOUD gave e0last 25,23,26; the STREAM gate gives 26,23,25).
        boolean hasEvents = false;
        for (JsonNode t : scenario.path("types")) {
            if (!t.path("event").isMissingNode()) hasEvents = true;
        }
        KieBase kbase;
        KieSession session;
        if (hasEvents) {
            kbase = new KieHelper().addContent(drl, ResourceType.DRL)
                    .build(org.kie.api.conf.EventProcessingOption.STREAM);
            org.kie.api.runtime.KieSessionConfiguration ksc =
                    org.kie.api.KieServices.Factory.get().newKieSessionConfiguration();
            ksc.setOption(org.kie.api.runtime.conf.ClockTypeOption.get("pseudo"));
            session = kbase.newKieSession(ksc, null);
        } else {
            kbase = new KieHelper().addContent(drl, ResourceType.DRL).build();
            session = kbase.newKieSession();
        }
        final int[] n = {0};
        session.addEventListener(new org.kie.api.event.rule.DefaultRuleRuntimeEventListener() {
            @Override
            public void objectInserted(org.kie.api.event.rule.ObjectInsertedEvent event) {
                System.out.println("WM-INSERT " + short_(event.getObject()));
                dumpAcc(session);
            }
            @Override
            public void objectUpdated(org.kie.api.event.rule.ObjectUpdatedEvent event) {
                System.out.println("WM-UPDATE " + short_(event.getObject()));
                dumpAcc(session);
            }
        });
        session.addEventListener(new DefaultAgendaEventListener() {
            @Override
            public void afterMatchFired(AfterMatchFiredEvent event) {
                StringBuilder sb = new StringBuilder();
                sb.append("FIRING ").append(n[0]++).append(' ')
                  .append(event.getMatch().getRule().getName()).append(' ');
                for (Object o : event.getMatch().getObjects()) {
                    sb.append(short_(o)).append(" | ");
                }
                System.out.println(sb);
                dumpAcc(session);
            }
        });
        java.util.List<org.kie.api.runtime.rule.FactHandle> inserted = new java.util.ArrayList<>();
        for (JsonNode fact : scenario.path("facts")) {
            inserted.add(session.insert(instantiate(kbase, scenario, fact)));
        }
        session.fireAllRules(10_000);
        System.out.println("== FIRE-BOUNDARY ==");
        dumpAcc(session);
        for (JsonNode epoch : scenario.path("epochs")) {
            // A2 winacc extension: replay epoch ACTIONS (advance/update/delete,
            // default-EP handles only) with a dump after each, mirroring
            // OracleRunner's property-masked ep.update.
            for (JsonNode action : epoch.path("actions")) {
                String op = action.path("op").asText();
                if (op.equals("advance")) {
                    ((org.drools.core.time.SessionPseudoClock) session.getSessionClock())
                            .advanceTime(action.path("ms").asLong(),
                                    java.util.concurrent.TimeUnit.MILLISECONDS);
                    System.out.println("-- ADVANCE " + action.path("ms").asLong());
                } else if (op.equals("update")) {
                    org.kie.api.runtime.rule.FactHandle fh =
                            inserted.get(action.path("target").asInt());
                    Object bean = session.getObject(fh);
                    FactType ft = kbase.getFactType(PKG, bean.getClass().getSimpleName());
                    java.util.List<String> props = new java.util.ArrayList<>();
                    java.util.Iterator<String> it = action.path("fields").fieldNames();
                    while (it.hasNext()) {
                        String fname = it.next();
                        JsonNode v = action.path("fields").path(fname);
                        String jt = null;
                        for (JsonNode t : scenario.path("types")) {
                            if (t.path("name").asText().equals(bean.getClass().getSimpleName())) {
                                for (JsonNode f : t.path("fields")) {
                                    if (f.path("name").asText().equals(fname)) {
                                        jt = f.path("type").asText();
                                    }
                                }
                            }
                        }
                        if ("i64".equals(jt)) ft.set(bean, fname, v.asLong());
                        else if ("f64".equals(jt)) ft.set(bean, fname, v.asDouble());
                        else if ("bool".equals(jt)) ft.set(bean, fname, v.asBoolean());
                        else ft.set(bean, fname, v.asText());
                        props.add(fname);
                    }
                    session.update(fh, bean, props.toArray(new String[0]));
                    System.out.println("-- UPDATE " + action.path("target").asInt()
                            + " " + action.path("fields"));
                } else if (op.equals("delete")) {
                    session.delete(inserted.get(action.path("target").asInt()));
                    System.out.println("-- DELETE " + action.path("target").asInt());
                } else {
                    System.out.println("-- UNSUPPORTED ACTION " + op);
                }
                dumpAcc(session);
            }
            for (JsonNode fact : epoch.path("facts")) {
                inserted.add(session.insert(instantiate(kbase, scenario, fact)));
            }
            session.fireAllRules(10_000);
            System.out.println("== FIRE-BOUNDARY ==");
            dumpAcc(session);
        }
        System.out.println("== QUIESCENT ==");
        dumpAcc(session);
        session.dispose();
    }

    static void dumpAcc(KieSession session) {
        try {
            Object reteEval = session; // StatefulKnowledgeSessionImpl implements ReteEvaluator
            Object kbase = call(session, "getKieBase");
            Object rete = call(kbase, "getRete");
            java.util.Map<?, ?> epsm = (java.util.Map<?, ?>) call(rete, "getEntryPointNodes");
            IdentityHashMap<Object, Boolean> seen = new IdentityHashMap<>();
            for (Object ep : epsm.values()) {
                java.util.Map<?, ?> otns = (java.util.Map<?, ?>) call(ep, "getObjectTypeNodes");
                for (Object otn : otns.values()) {
                    walk(otn, reteEval, seen);
                }
            }
        } catch (Throwable t) {
            System.out.println("  dump error: " + t);
        }
    }

    static void walk(Object node, Object reteEval, IdentityHashMap<Object, Boolean> seen) throws Exception {
        if (node == null || seen.put(node, true) != null) return;
        if (node.getClass().getSimpleName().equals("RuleTerminalNode")) {
            try {
                Object pmem = call1(reteEval, "getNodeMemory", node,
                        Class.forName("org.drools.core.common.MemoryFactory"));
                Object item = call(pmem, "getRuleAgendaItem");
                StringBuilder sb = new StringBuilder("  RTN ");
                sb.append(call(call(node, "getRule"), "getName"));
                sb.append(" lmask=").append(call(pmem, "getLinkedSegmentMask"));
                sb.append("/").append(call(pmem, "getAllLinkedMaskTest"));
                if (item != null) {
                    sb.append(" item[queued=").append(call(item, "isQueued"));
                    Object ex = call(item, "getRuleExecutor");
                    if (ex != null) sb.append(" dirty=").append(call(ex, "isDirty"));
                    sb.append(']');
                } else {
                    sb.append(" item=null");
                }
                System.out.println(sb);
            } catch (Throwable t) {
                System.out.println("  RTN dump error: " + t);
            }
        }
        if (node.getClass().getSimpleName().contains("AccumulateNode")) {
            dumpAccNode(node, reteEval);
        } else if (node.getClass().getSimpleName().equals("JoinNode")) {
            dumpJoinNode(node, reteEval);
        } else if (node.getClass().getSimpleName().equals("WindowNode")) {
            dumpWindowNode(node, reteEval);
        }
        // descend object sinks and left sinks
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

    /** A2 winacc: the SlidingTimeWindow queue — each queued CLONE handle's
     *  bean + startTimestamp + validity (the window-membership ground truth). */
    static void dumpWindowNode(Object node, Object reteEval) throws Exception {
        Object mem = call1(reteEval, "getNodeMemory", node,
                Class.forName("org.drools.core.common.MemoryFactory"));
        StringBuilder sb = new StringBuilder("  WIN node ").append(call(node, "getId"));
        sb.append(" queue[");
        Object ctxs = mem.getClass().getField("behaviorContext").get(mem);
        for (Object ctx : (Object[]) ctxs) {
            for (Object efh : (java.util.Collection<?>) call(ctx, "getFactHandles")) {
                sb.append(short_(call(efh, "getObject")))
                        .append("@ts").append(call(efh, "getStartTimestamp"))
                        .append(call(efh, "isValid").equals(Boolean.TRUE) ? "" : "!INVALID")
                        .append("; ");
            }
        }
        sb.append(']');
        System.out.println(sb);
    }

    static void dumpJoinNode(Object node, Object reteEval) throws Exception {
        Object bm = call1(reteEval, "getNodeMemory", node,
                Class.forName("org.drools.core.common.MemoryFactory"));
        Object ltm = call(bm, "getLeftTupleMemory");
        Object rtm = call(bm, "getRightTupleMemory");
        Object srt = call(bm, "getStagedRightTuples");
        StringBuilder sb = new StringBuilder("  JOIN ").append(call(node, "getId"));
        sb.append(" rtm[");
        Object rit = call(rtm, "iterator");
        for (Object rt = call(rit, "next"); rt != null; rt = call(rit, "next")) {
            Object fh = call(rt, "getFactHandle");
            sb.append(short_(fh != null ? call(fh, "getObject") : null)).append("; ");
        }
        sb.append("] stagedR-ins[");
        Object si = call(srt, "getInsertFirst");
        while (si != null) {
            Object fh = call(si, "getFactHandle");
            sb.append(short_(fh != null ? call(fh, "getObject") : null)).append("; ");
            si = call(si, "getStagedNext");
        }
        sb.append("] ltm[");
        Object lit = call(ltm, "iterator");
        for (Object lt = call(lit, "next"); lt != null; lt = call(lit, "next")) {
            sb.append(tupleFacts(lt)).append("; ");
        }
        sb.append(']');
        System.out.println(sb);
    }

    /** Render a left tuple as its fact chain, root->leaf, each fact's ts.
     *  The opaque LeftTuple.toString hides the joined facts; for the join-
     *  order port we need the ACTUAL (E0,E1..) sequence node memory holds. */
    static String tupleFacts(Object tuple) throws Exception {
        java.util.ArrayList<String> facts = new java.util.ArrayList<>();
        Object t = tuple;
        while (t != null) {
            Object fh = null;
            try { fh = call(t, "getFactHandle"); } catch (Exception ignore) { }
            if (fh != null) facts.add(short_(call(fh, "getObject")));
            Object parent = null;
            try { parent = call(t, "getParent"); } catch (Exception ignore) { }
            t = parent;
        }
        java.util.Collections.reverse(facts);
        return "(" + String.join(",", facts) + ")";
    }

    static void dumpAccNode(Object accNode, Object reteEval) throws Exception {
        Object mem = call1(reteEval, "getNodeMemory", accNode,
                Class.forName("org.drools.core.common.MemoryFactory"));
        Object bm = call(mem, "getBetaMemory");
        Object ltm = call(bm, "getLeftTupleMemory");
        Object rtm = call(bm, "getRightTupleMemory");
        System.out.println("  ACC node " + call(accNode, "getId"));
        // right memory order
        StringBuilder rb = new StringBuilder("   rtm: ");
        Object rit = call(rtm, "iterator");
        for (Object rt = call(rit, "next"); rt != null; rt = call(rit, "next")) {
            Object fh = call(rt, "getFactHandle");
            rb.append(short_(fh != null ? call(fh, "getObject") : null)).append("  ");
        }
        System.out.println(rb);
        Object it = call(ltm, "iterator");
        for (Object ltx = call(it, "next"); ltx != null; ltx = call(it, "next")) {
            Object lt = ltx;
            Object leftFact = call(lt, "getFactHandle") != null
                    ? call(call(lt, "getFactHandle"), "getObject") : null;
            Object ctx = call(lt, "getContextObject");
            StringBuilder sb = new StringBuilder("   left ").append(short_(leftFact));
            if (ctx != null) {
                sb.append("  fctx{");
                dumpFields(ctx, sb);
                sb.append("}");
                try {
                    Object rfh = call(ctx, "getResultFactHandle");
                    if (rfh != null) sb.append(" RESULT=").append(short_(call(rfh, "getObject")));
                } catch (Exception ignore) { }
                try {
                    Object fc = call(ctx, "getFunctionContext");
                    if (fc != null) {
                        sb.append(" fn{");
                        if (fc instanceof Object[]) {
                            for (Object e : (Object[]) fc) {
                                if (e != null) dumpFields(e, sb);
                            }
                        } else {
                            dumpFields(fc, sb);
                        }
                        sb.append('}');
                    }
                } catch (Exception ignore) { }
            }
            sb.append("  matches:");
            Object child = call(lt, "getFirstChild");
            while (child != null) {
                Object rp = call(child, "getRightParent");
                Object rf = rp != null ? call(call(rp, "getFactHandleForEvaluation"), "getObject") : null;
                Object cctx = call(child, "getContextObject");
                sb.append(" [").append(short_(rf));
                if (cctx != null) sb.append(" c=").append(cctx);
                sb.append("]");
                child = call(child, "getHandleNext");
            }
            System.out.println(sb);
        }
    }

    static void dumpFields(Object o, StringBuilder sb) throws Exception {
        for (Class<?> c = o.getClass(); c != null && c != Object.class; c = c.getSuperclass()) {
            for (Field f : c.getDeclaredFields()) {
                f.setAccessible(true);
                Object v = f.get(o);
                if (v == null) continue;
                if (v instanceof Object[]) {
                    Object[] arr = (Object[]) v;
                    sb.append(f.getName()).append("=[");
                    for (Object e : arr) {
                        if (e == null) continue;
                        sb.append(e.getClass().getSimpleName()).append(':');
                        dumpFields(e, sb);
                        sb.append(' ');
                    }
                    sb.append("] ");
                } else if (v instanceof Number || v instanceof Boolean || v instanceof Comparable) {
                    sb.append(f.getName()).append('=').append(v).append(' ');
                }
            }
        }
    }

    static String short_(Object o) {
        if (o == null) return "null";
        String s = String.valueOf(o);
        return s.length() > 60 ? s.substring(0, 60) : s;
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
        // fallback: any single-arg overload accepting the object
        for (Method mm : o.getClass().getMethods()) {
            if (mm.getName().equals(m) && mm.getParameterCount() == 1
                    && mm.getParameterTypes()[0].isInstance(arg)) {
                mm.setAccessible(true);
                return mm.invoke(o, arg);
            }
        }
        throw new NoSuchMethodException(m);
    }

    // -- minimal copies of the runner's declare/instantiate helpers --
    // Must mirror OracleRunner.declareBlocks EXACTLY, incl. the CEP event
    // annotations (@role/@timestamp/@duration/@expires) — otherwise temporal
    // scenarios declare their types as plain facts and the temporal evaluator
    // throws DefaultFactHandle-cannot-be-cast-to-EventHandle at fire time.
    static String declareBlocks(JsonNode types) {
        StringBuilder sb = new StringBuilder();
        for (JsonNode t : types) {
            sb.append("declare ").append(t.path("name").asText()).append('\n');
            JsonNode ev = t.path("event");
            if (!ev.isMissingNode()) {
                sb.append("    @role( event )\n");
                if (ev.has("timestamp")) {
                    sb.append("    @timestamp( ").append(ev.path("timestamp").asText()).append(" )\n");
                }
                if (ev.has("duration")) {
                    sb.append("    @duration( ").append(ev.path("duration").asText()).append(" )\n");
                }
                if (ev.has("expires_ms")) {
                    sb.append("    @expires( ").append(ev.path("expires_ms").asLong()).append("ms )\n");
                }
            }
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
        String tname = fact.path("type").asText();
        FactType ft = kbase.getFactType(PKG, tname);
        Object bean = ft.newInstance();
        java.util.Iterator<String> it = fact.path("fields").fieldNames();
        while (it.hasNext()) {
            String fname = it.next();
            JsonNode v = fact.path("fields").path(fname);
            String jt = null;
            for (JsonNode t : scenario.path("types")) {
                if (t.path("name").asText().equals(tname)) {
                    for (JsonNode f : t.path("fields")) {
                        if (f.path("name").asText().equals(fname)) {
                            jt = f.path("type").asText();
                        }
                    }
                }
            }
            if ("i64".equals(jt)) ft.set(bean, fname, v.asLong());
            else if ("f64".equals(jt)) ft.set(bean, fname, v.asDouble());
            else if ("bool".equals(jt)) ft.set(bean, fname, v.asBoolean());
            else ft.set(bean, fname, v.asText());
        }
        return bean;
    }
}
