package dev.seine.oracle;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.kie.api.KieBase;
import org.kie.api.definition.type.FactType;
import org.kie.api.event.rule.AfterMatchFiredEvent;
import org.kie.api.event.rule.DefaultAgendaEventListener;
import org.kie.api.io.ResourceType;
import org.kie.api.runtime.KieSession;
import org.kie.api.runtime.rule.FactHandle;
import org.kie.internal.utils.KieHelper;

import java.lang.reflect.Method;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Locale;
import java.util.TimeZone;

/**
 * D-150 graft (bf-with-arrivals recon, RunnerDump pattern, modeled on
 * ExistsDump): a seq-numbered DYNAMIC event stream on the not→join path.
 * Every WM event (insert/update/delete — expiry retractions included),
 * every agenda event (created/cancelled/fired) WITH tuple facts, and
 * main-loop markers (actions, epoch/fire boundaries), interleaved with
 * state dumps: per-beta-node right-memory ORDER + staged R ins/del/upd
 * (+ blocked lefts on nots), per-segment staged lefts, and the
 * RuleExecutor tupleList — the firing queue — in order. The point: watch
 * WHERE an external P update lands in the queue relative to the unblock
 * re-propagations, per arrival, mid-fire. Diagnostic only; never part of
 * the gate.
 *
 * Usage: java ... dev.seine.oracle.BfDump <scenario.json>
 */
public final class BfDump {

    private static final String PKG = "seine.gen";
    private static final ObjectMapper M = new ObjectMapper();
    private static int seq = 0;
    private static int fireNo = 0;
    private static KieSession SESSION;
    private static final List<Object> RTNS = new ArrayList<>();
    private static final List<Object> BETAS = new ArrayList<>();

    static void log(String s) {
        System.out.println("[" + (seq++) + "] " + s);
    }

    public static void main(String[] args) throws Exception {
        Locale.setDefault(new Locale("en", "US"));
        TimeZone.setDefault(TimeZone.getTimeZone("UTC"));
        JsonNode scenario = M.readTree(Files.readString(Path.of(args[0])));
        String drl = "package " + PKG + ";\n"
                + declareBlocks(scenario.path("types")) + "\n" + scenario.path("drl").asText();
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
        SESSION = session;
        collectNodes(session);
        wrapPropagationList(session);

        final List<FactHandle> inserted = new ArrayList<>();
        session.addEventListener(new org.kie.api.event.rule.DefaultRuleRuntimeEventListener() {
            @Override
            public void objectInserted(org.kie.api.event.rule.ObjectInsertedEvent event) {
                if (!event.getObject().getClass().getSimpleName().equals("InitialFactImpl")) {
                    inserted.add(event.getFactHandle());
                    log("WM+INSERT " + label(event.getObject()) + "  h" + (inserted.size() - 1));
                    dumpQueues("");
                }
            }
            @Override
            public void objectUpdated(org.kie.api.event.rule.ObjectUpdatedEvent event) {
                log("WM~UPDATE " + label(event.getObject()));
                dumpQueues("");
            }
            @Override
            public void objectDeleted(org.kie.api.event.rule.ObjectDeletedEvent event) {
                log("WM-DELETE " + label(event.getOldObject()));
                dumpQueues("");
            }
        });
        session.addEventListener(new DefaultAgendaEventListener() {
            @Override
            public void matchCreated(org.kie.api.event.rule.MatchCreatedEvent event) {
                log("MATCH+ " + event.getMatch().getRule().getName() + " " + matchFacts(event.getMatch()));
                dumpQueues("");
            }
            @Override
            public void matchCancelled(org.kie.api.event.rule.MatchCancelledEvent event) {
                log("MATCH- " + event.getMatch().getRule().getName() + " " + matchFacts(event.getMatch())
                        + " cause=" + event.getCause());
                dumpQueues("");
            }
            @Override
            public void afterMatchFired(AfterMatchFiredEvent event) {
                log("FIRE " + (fireNo++) + " " + event.getMatch().getRule().getName()
                        + " " + matchFacts(event.getMatch()));
                dumpQueues("");
            }
        });

        for (JsonNode fact : scenario.path("facts")) {
            log("== ACTION init-insert " + fact.path("type").asText() + " " + fact.path("fields"));
            session.insert(instantiate(kbase, scenario, fact));
        }
        log("== PRE-FIRE (initial)  clock=" + clock(session));
        dumpState();
        session.fireAllRules(10_000);
        log("== FIRE-BOUNDARY (initial)");
        dumpState();

        int epNo = 0;
        for (JsonNode epoch : scenario.path("epochs")) {
            epNo++;
            for (JsonNode action : epoch.path("actions")) {
                String op = action.path("op").asText();
                if (op.equals("insert")) {
                    log("== ACTION insert " + action.path("type").asText() + " " + action.path("fields") + " (ep " + epNo + ")");
                    session.insert(instantiate(kbase, scenario, action));
                } else if (op.equals("update")) {
                    int target = action.path("target").asInt();
                    FactHandle fh = inserted.get(target);
                    Object bean = session.getObject(fh);
                    log("== ACTION update h" + target + "=" + label(bean) + " <- " + action.path("fields") + " (ep " + epNo + ")");
                    FactType ft = kbase.getFactType(PKG, bean.getClass().getSimpleName());
                    List<String> props = new ArrayList<>();
                    java.util.Iterator<String> it = action.path("fields").fieldNames();
                    while (it.hasNext()) {
                        String fname = it.next();
                        JsonNode v = action.path("fields").path(fname);
                        setTyped(ft, bean, fname, v, scenario);
                        props.add(fname);
                    }
                    session.update(fh, bean, props.toArray(new String[0]));
                } else if (op.equals("delete")) {
                    int target = action.path("target").asInt();
                    log("== ACTION delete h" + target + " (ep " + epNo + ")");
                    session.delete(inserted.get(target));
                } else if (op.equals("advance")) {
                    log("== ACTION advance " + action.path("ms").asLong() + "ms (ep " + epNo + ")  clock=" + clock(session));
                    ((org.drools.core.time.SessionPseudoClock) session.getSessionClock())
                            .advanceTime(action.path("ms").asLong(), java.util.concurrent.TimeUnit.MILLISECONDS);
                    log("== (advance done)  clock=" + clock(session));
                }
                dumpState();
            }
            for (JsonNode fact : epoch.path("facts")) {
                log("== FACT insert " + fact.path("type").asText() + " " + fact.path("fields") + " (ep " + epNo + ")");
                session.insert(instantiate(kbase, scenario, fact));
                dumpState();
            }
            log("== PRE-FIRE (ep " + epNo + ")  clock=" + clock(session));
            dumpState();
            session.fireAllRules(10_000);
            log("== FIRE-BOUNDARY (ep " + epNo + ")");
            dumpState();
        }
        session.dispose();
    }

    static String clock(KieSession s) {
        try { return String.valueOf(((org.drools.core.time.SessionPseudoClock) s.getSessionClock()).getCurrentTime()); }
        catch (Exception e) { return "?"; }
    }

    // ---- propagation-queue instrumentation ----

    /** Swap the ActivationsManager's PropagationList with a logging proxy:
     *  logs every addEntry (enqueue) and, by wrapping the chain returned from
     *  takeAll, every entry EXECUTION — with a state dump after each. */
    static void wrapPropagationList(KieSession session) throws Exception {
        Object am = call(session, "getActivationsManager");
        java.lang.reflect.Field f = null;
        for (Class<?> c = am.getClass(); c != null; c = c.getSuperclass()) {
            try { f = c.getDeclaredField("propagationList"); break; } catch (NoSuchFieldException ignore) { }
        }
        f.setAccessible(true);
        final Object orig = f.get(am);
        Class<?> plIface = Class.forName("org.drools.core.phreak.PropagationList");
        Object proxy = java.lang.reflect.Proxy.newProxyInstance(plIface.getClassLoader(), new Class<?>[]{plIface},
                (p, m, margs) -> {
                    if (m.getName().equals("addEntry")) {
                        log("QUEUE+ " + entryLabel(margs[0]));
                        return m.invoke(orig, margs);
                    }
                    Object r = m.invoke(orig, margs);
                    if (m.getName().equals("takeAll") && r != null) {
                        log("TAKEALL");
                        return wrapEntry(r);
                    }
                    return r;
                });
        f.set(am, proxy);
    }

    static Object wrapEntry(Object entry) throws Exception {
        if (entry == null || java.lang.reflect.Proxy.isProxyClass(entry.getClass())) return entry;
        Class<?> peIface = Class.forName("org.drools.core.phreak.PropagationEntry");
        return java.lang.reflect.Proxy.newProxyInstance(peIface.getClassLoader(), new Class<?>[]{peIface},
                (p, m, margs) -> {
                    switch (m.getName()) {
                        case "execute": {
                            log("EXEC> " + entryLabel(entry));
                            Object rr = m.invoke(entry, margs);
                            dumpQueues("");
                            return rr;
                        }
                        case "getNext":
                            return wrapEntry(m.invoke(entry, margs));
                        default:
                            return m.invoke(entry, margs);
                    }
                });
    }

    static String entryLabel(Object entry) {
        String cn = entry.getClass().getSimpleName();
        StringBuilder sb = new StringBuilder(cn);
        for (String fld : new String[]{"handle", "factHandle"}) {
            try {
                java.lang.reflect.Field hf = entry.getClass().getDeclaredField(fld);
                hf.setAccessible(true);
                Object h = hf.get(entry);
                if (h != null) sb.append(' ').append(label(call(h, "getObject")));
                break;
            } catch (Exception ignore) { }
        }
        if (cn.contains("Expire")) sb.append(" [").append(label(entry)).append(']');
        return sb.toString();
    }

    // ---- node discovery ----

    static void collectNodes(KieSession session) throws Exception {
        Object kbase = call(session, "getKieBase");
        Object rete = call(kbase, "getRete");
        java.util.Map<?, ?> epsm = (java.util.Map<?, ?>) call(rete, "getEntryPointNodes");
        IdentityHashMap<Object, Boolean> seen = new IdentityHashMap<>();
        for (Object ep : epsm.values()) {
            java.util.Map<?, ?> otns = (java.util.Map<?, ?>) call(ep, "getObjectTypeNodes");
            for (Object otn : otns.values()) walk(otn, seen);
        }
    }

    static void walk(Object node, IdentityHashMap<Object, Boolean> seen) throws Exception {
        if (node == null || seen.put(node, true) != null) return;
        String cn = node.getClass().getSimpleName();
        if (cn.equals("NotNode") || cn.equals("ExistsNode") || cn.equals("JoinNode")) BETAS.add(node);
        if (cn.equals("RuleTerminalNode")) RTNS.add(node);
        for (String prop : new String[]{"getObjectSinkPropagator", "getSinkPropagator"}) {
            try {
                Object propag = call(node, prop);
                if (propag == null) continue;
                Object[] sinks = (Object[]) call(propag, "getSinks");
                for (Object s : sinks) walk(s, seen);
            } catch (NoSuchMethodException ignore) {
            }
        }
    }

    // ---- state dumps ----

    /** Per-event dump: beta staged/memories + executor queues (mid-fire visibility). */
    static void dumpQueues(String indent) {
        try {
            for (Object node : BETAS) dumpBeta(node);
        } catch (Throwable ignore) {
        }
        for (Object rtn : RTNS) {
            try {
                String q = queueString(rtn);
                if (q != null) System.out.println("      " + indent + "Q=" + q);
            } catch (Throwable ignore) {
            }
        }
    }

    static String queueString(Object rtn) throws Exception {
        Object pmem = call1(SESSION, "getNodeMemory", rtn, Class.forName("org.drools.core.common.MemoryFactory"));
        Object rai = callOrNull(pmem, "getRuleAgendaItem");
        if (rai == null) return "<no-rai>";
        Object ex = callOrNull(rai, "getRuleExecutor");
        if (ex == null) return "<no-executor>";
        Object tl = call(ex, "getLeftTupleList");
        StringBuilder sb = new StringBuilder("[");
        for (Object t = callOrNull(tl, "getFirst"); t != null; t = callOrNull(t, "getNext")) {
            sb.append(tupleFacts(t));
            Object q = callOrNull(t, "isQueued");
            if (!Boolean.TRUE.equals(q)) sb.append("!q");
            sb.append(' ');
        }
        return sb.append(']').toString();
    }

    /** Full: beta memories + staged + segment staging + queues. */
    static void dumpState() {
        try {
            for (Object node : BETAS) dumpBeta(node);
            for (Object rtn : RTNS) {
                Object pmem = call1(SESSION, "getNodeMemory", rtn, Class.forName("org.drools.core.common.MemoryFactory"));
                StringBuilder sb = new StringBuilder("      RTN ").append(call(rtn, "getId"));
                Object[] smems = (Object[]) callOrNull(pmem, "getSegmentMemories");
                if (smems != null) {
                    int i = 0;
                    for (Object smem : smems) {
                        sb.append("\n        smem[").append(i++).append("] ");
                        if (smem == null) { sb.append("null"); continue; }
                        Object staged = callOrNull(smem, "getStagedLeftTuples");
                        sb.append(stagedString(staged));
                    }
                }
                sb.append("\n        Q=").append(queueString(rtn));
                System.out.println(sb);
            }
        } catch (Throwable t) {
            System.out.println("      dump error: " + t);
        }
    }

    static void dumpBeta(Object node) throws Exception {
        String cn = node.getClass().getSimpleName();
        Object bm = call1(SESSION, "getNodeMemory", node, Class.forName("org.drools.core.common.MemoryFactory"));
        Object ltm = call(bm, "getLeftTupleMemory");
        Object rtm = call(bm, "getRightTupleMemory");
        Object srt = call(bm, "getStagedRightTuples");
        StringBuilder sb = new StringBuilder("      ").append(cn).append(' ').append(call(node, "getId"));
        sb.append(" rtm=[");
        Object rit = call(rtm, "iterator");
        for (Object rt = call(rit, "next"); rt != null; rt = call(rit, "next")) {
            sb.append(rightLabel(rt));
            if (cn.equals("NotNode") || cn.equals("ExistsNode")) {
                Object blocked = callOrNull(rt, "getBlocked");
                if (blocked != null) {
                    sb.append("blocked{");
                    for (Object lt = blocked; lt != null; lt = callOrNull(lt, "getBlockedNext")) {
                        sb.append(tupleFacts(lt)).append(hasChild(lt)).append(' ');
                    }
                    sb.append('}');
                }
            }
            sb.append(' ');
        }
        sb.append("] stagedR=").append(stagedString(srt));
        sb.append(" ltm=[");
        Object lit = call(ltm, "iterator");
        for (Object lt = call(lit, "next"); lt != null; lt = call(lit, "next")) {
            sb.append(tupleFacts(lt)).append(hasChild(lt)).append(' ');
        }
        sb.append(']');
        System.out.println(sb);
    }

    static String stagedString(Object tupleSets) {
        if (tupleSets == null) return "<null>";
        StringBuilder sb = new StringBuilder("ins[");
        for (Object s = callOrNull(tupleSets, "getInsertFirst"); s != null; s = callOrNull(s, "getStagedNext"))
            sb.append(anyTupleLabel(s)).append(' ');
        sb.append("] del[");
        for (Object s = callOrNull(tupleSets, "getDeleteFirst"); s != null; s = callOrNull(s, "getStagedNext"))
            sb.append(anyTupleLabel(s)).append(' ');
        sb.append("] upd[");
        for (Object s = callOrNull(tupleSets, "getUpdateFirst"); s != null; s = callOrNull(s, "getStagedNext"))
            sb.append(anyTupleLabel(s)).append(' ');
        return sb.append(']').toString();
    }

    /** Works for both right tuples (single fact) and left tuples (chains). */
    static String anyTupleLabel(Object t) {
        try {
            String cn = t.getClass().getSimpleName();
            if (cn.contains("RightTuple")) return rightLabel(t);
            return tupleFacts(t);
        } catch (Exception e) {
            return "?";
        }
    }

    static String matchFacts(org.kie.api.runtime.rule.Match m) {
        StringBuilder sb = new StringBuilder("(");
        for (Object o : m.getObjects()) sb.append(label(o)).append(',');
        if (sb.charAt(sb.length() - 1) == ',') sb.setLength(sb.length() - 1);
        return sb.append(')').toString();
    }

    static String rightLabel(Object rt) {
        try {
            Object fh = callOrNull(rt, "getFactHandleForEvaluation");
            if (fh == null) fh = callOrNull(rt, "getFactHandle");
            String o = fh != null ? label(call(fh, "getObject")) : "?";
            String del = "";
            try { Object d = callOrNull(rt, "isDeleted"); if (Boolean.TRUE.equals(d)) del = "!DEL"; } catch (Exception ignore) {}
            return o + del;
        } catch (Exception e) { return "?"; }
    }

    static String hasChild(Object lt) {
        try { return callOrNull(lt, "getFirstChild") != null ? "*" : ""; } catch (Exception e) { return "?"; }
    }

    static String tupleFacts(Object tuple) throws Exception {
        java.util.ArrayList<String> facts = new java.util.ArrayList<>();
        Object t = tuple;
        while (t != null) {
            Object fh = null;
            try { fh = call(t, "getFactHandle"); } catch (Exception ignore) { }
            if (fh != null && call(fh, "getObject") != null
                    && !call(fh, "getObject").getClass().getSimpleName().equals("InitialFactImpl"))
                facts.add(label(call(fh, "getObject")));
            Object parent = null;
            try { parent = call(t, "getParent"); } catch (Exception ignore) { }
            t = parent;
        }
        java.util.Collections.reverse(facts);
        return "(" + String.join(",", facts) + ")";
    }

    /** "P( v=3 )" -> "P(v=3)" — compact bean label. */
    static String label(Object o) {
        if (o == null) return "null";
        String s = String.valueOf(o).replace(" ", "");
        return s.length() > 40 ? s.substring(0, 40) : s;
    }

    // ---- reflection helpers (ExistsDump pattern) ----

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

    static Object callOrNull(Object o, String m) {
        if (o == null) return null;
        try { return call(o, m); } catch (Exception e) { return null; }
    }

    static Object call1(Object o, String m, Object arg, Class<?> ptype) throws Exception {
        for (Method mm : o.getClass().getMethods()) {
            if (mm.getName().equals(m) && mm.getParameterCount() == 1
                    && mm.getParameterTypes()[0].isAssignableFrom(ptype)) {
                mm.setAccessible(true);
                return mm.invoke(o, arg);
            }
        }
        for (Method mm : o.getClass().getMethods()) {
            if (mm.getName().equals(m) && mm.getParameterCount() == 1
                    && mm.getParameterTypes()[0].isInstance(arg)) {
                mm.setAccessible(true);
                return mm.invoke(o, arg);
            }
        }
        throw new NoSuchMethodException(m);
    }

    static void setTyped(FactType ft, Object bean, String fname, JsonNode v, JsonNode scenario) throws Exception {
        String tname = bean.getClass().getSimpleName();
        String jt = null;
        for (JsonNode t : scenario.path("types")) {
            if (t.path("name").asText().equals(tname)) {
                for (JsonNode f : t.path("fields")) {
                    if (f.path("name").asText().equals(fname)) jt = f.path("type").asText();
                }
            }
        }
        if ("i64".equals(jt)) ft.set(bean, fname, v.asLong());
        else if ("f64".equals(jt)) ft.set(bean, fname, v.asDouble());
        else if ("bool".equals(jt)) ft.set(bean, fname, v.asBoolean());
        else ft.set(bean, fname, v.asText());
    }

    static String declareBlocks(JsonNode types) {
        StringBuilder sb = new StringBuilder();
        for (JsonNode t : types) {
            sb.append("declare ").append(t.path("name").asText()).append('\n');
            JsonNode ev = t.path("event");
            if (!ev.isMissingNode()) {
                sb.append("    @role( event )\n");
                if (ev.has("timestamp"))
                    sb.append("    @timestamp( ").append(ev.path("timestamp").asText()).append(" )\n");
                if (ev.has("duration"))
                    sb.append("    @duration( ").append(ev.path("duration").asText()).append(" )\n");
                if (ev.has("expires_ms"))
                    sb.append("    @expires( ").append(ev.path("expires_ms").asLong()).append("ms )\n");
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
            setTyped(ft, bean, fname, v, scenario);
        }
        return bean;
    }
}
