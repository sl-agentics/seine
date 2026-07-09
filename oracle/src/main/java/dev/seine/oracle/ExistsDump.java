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
 * CLASS-3 sub-recon graft (RunnerDump pattern, modeled on AccDump): dumps
 * every Exists/Not BetaMemory — right memory with each right tuple's BLOCKED
 * left list, the unblocked left memory, the staged right tuples (ins/del), and
 * each left's blocker + first child (the propagated activation) — AFTER EACH
 * epoch action (delete/insert/advance) and after fireAllRules. The point: see
 * whether an external EVENT delete of a blocker un-blocks the exists
 * IMMEDIATELY (before the same-epoch reinsert) vs a plain fact's batched
 * staging. Diagnostic only; never part of the gate.
 *
 * Usage: java ... dev.seine.oracle.ExistsDump <scenario.json>
 */
public final class ExistsDump {

    private static final String PKG = "seine.gen";
    private static final ObjectMapper M = new ObjectMapper();
    private static int fireNo = 0;

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

        final List<FactHandle> inserted = new ArrayList<>();
        session.addEventListener(new org.kie.api.event.rule.DefaultRuleRuntimeEventListener() {
            @Override
            public void objectInserted(org.kie.api.event.rule.ObjectInsertedEvent event) {
                if (!event.getObject().getClass().getSimpleName().equals("InitialFactImpl")) {
                    inserted.add(event.getFactHandle());
                }
            }
        });
        session.addEventListener(new DefaultAgendaEventListener() {
            @Override
            public void afterMatchFired(AfterMatchFiredEvent event) {
                StringBuilder sb = new StringBuilder("FIRING ").append(fireNo++).append(' ')
                        .append(event.getMatch().getRule().getName()).append("  ");
                for (Object o : event.getMatch().getObjects()) sb.append(short_(o)).append(" | ");
                System.out.println(sb);
            }
            @Override
            public void matchCreated(org.kie.api.event.rule.MatchCreatedEvent event) {
                System.out.println("  match+CREATED " + event.getMatch().getRule().getName());
            }
            @Override
            public void matchCancelled(org.kie.api.event.rule.MatchCancelledEvent event) {
                System.out.println("  match-CANCELLED " + event.getMatch().getRule().getName()
                        + " cause=" + event.getCause());
            }
        });
        session.addEventListener(new org.kie.api.event.rule.DefaultRuleRuntimeEventListener() {
            @Override
            public void objectDeleted(org.kie.api.event.rule.ObjectDeletedEvent e) {
                System.out.println("  WM-DELETE " + short_(e.getOldObject()));
            }
        });

        for (JsonNode fact : scenario.path("facts")) {
            session.insert(instantiate(kbase, scenario, fact));
        }
        session.fireAllRules(10_000);
        System.out.println("== INITIAL FIRE-BOUNDARY ==");
        dumpExists(session);

        for (JsonNode epoch : scenario.path("epochs")) {
            for (JsonNode action : epoch.path("actions")) {
                String op = action.path("op").asText();
                if (op.equals("insert")) {
                    session.insert(instantiate(kbase, scenario, action));
                    System.out.println("-- after ACTION insert " + action.path("fields") + " (NOT yet fired) --");
                } else if (op.equals("delete")) {
                    int target = action.path("target").asInt();
                    session.delete(inserted.get(target));
                    System.out.println("-- after ACTION delete target=" + target + " (NOT yet fired) --");
                } else if (op.equals("advance")) {
                    ((org.drools.core.time.SessionPseudoClock) session.getSessionClock())
                            .advanceTime(action.path("ms").asLong(), java.util.concurrent.TimeUnit.MILLISECONDS);
                    System.out.println("-- after ACTION advance " + action.path("ms").asLong() + "ms (NOT yet fired) --");
                }
                dumpExists(session);
            }
            for (JsonNode fact : epoch.path("facts")) {
                session.insert(instantiate(kbase, scenario, fact));
                System.out.println("-- after FACT insert " + fact.path("fields") + " (NOT yet fired) --");
                dumpExists(session);
            }
            session.fireAllRules(10_000);
            System.out.println("== EPOCH FIRE-BOUNDARY ==");
            dumpExists(session);
        }
        session.dispose();
    }

    static void dumpExists(KieSession session) {
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
        if (cn.equals("ExistsNode") || cn.equals("NotNode")) {
            dumpBetaExistential(node, reteEval, cn);
        }
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

    static void dumpBetaExistential(Object node, Object reteEval, String cn) throws Exception {
        Object bm = call1(reteEval, "getNodeMemory", node,
                Class.forName("org.drools.core.common.MemoryFactory"));
        Object ltm = call(bm, "getLeftTupleMemory");
        Object rtm = call(bm, "getRightTupleMemory");
        Object srt = call(bm, "getStagedRightTuples");
        StringBuilder sb = new StringBuilder("  ").append(cn).append(' ').append(call(node, "getId"));

        // right memory: each right + its BLOCKED left list
        sb.append("\n     rtm: ");
        Object rit = call(rtm, "iterator");
        for (Object rt = call(rit, "next"); rt != null; rt = call(rit, "next")) {
            sb.append(rightLabel(rt));
            Object blocked = callOrNull(rt, "getBlocked"); // first blocked LeftTuple
            sb.append(" blocked{");
            for (Object lt = blocked; lt != null; lt = callOrNull(lt, "getBlockedNext")) {
                sb.append(tupleFacts(lt)).append(hasChild(lt)).append(' ');
            }
            sb.append("}  ");
        }
        // staged right tuples (ins/del) present between an action and the fire
        sb.append("\n     stagedR ins[");
        for (Object s = callOrNull(srt, "getInsertFirst"); s != null; s = callOrNull(s, "getStagedNext"))
            sb.append(rightLabel(s)).append(' ');
        sb.append("] del[");
        for (Object s = callOrNull(srt, "getDeleteFirst"); s != null; s = callOrNull(s, "getStagedNext"))
            sb.append(rightLabel(s)).append(' ');
        sb.append("] upd[");
        for (Object s = callOrNull(srt, "getUpdateFirst"); s != null; s = callOrNull(s, "getStagedNext"))
            sb.append(rightLabel(s)).append(' ');
        sb.append(']');
        // left memory: UNBLOCKED lefts (blocked lefts live on the blocker)
        sb.append("\n     ltm(unblocked): ");
        Object lit = call(ltm, "iterator");
        for (Object lt = call(lit, "next"); lt != null; lt = call(lit, "next")) {
            sb.append(tupleFacts(lt)).append(hasChild(lt)).append(' ');
        }
        System.out.println(sb);
    }

    static String rightLabel(Object rt) {
        try {
            Object fh = callOrNull(rt, "getFactHandleForEvaluation");
            if (fh == null) fh = callOrNull(rt, "getFactHandle");
            String o = fh != null ? short_(call(fh, "getObject")) : "?";
            String del = "";
            try { Object d = callOrNull(rt, "isDeleted"); if (Boolean.TRUE.equals(d)) del = "!DEL"; } catch (Exception ignore) {}
            return o + del + "; ";
        } catch (Exception e) { return "?; "; }
    }

    /** "*" if this left has a first child (a propagated activation tuple). */
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
                facts.add(short_(call(fh, "getObject")));
            Object parent = null;
            try { parent = call(t, "getParent"); } catch (Exception ignore) { }
            t = parent;
        }
        java.util.Collections.reverse(facts);
        return "(" + String.join(",", facts) + ")";
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

    static Object callOrNull(Object o, String m) {
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
        return bean;
    }
}
