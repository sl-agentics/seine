package dev.seine.oracle;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.kie.api.KieBase;
import org.kie.api.definition.type.FactType;
import org.kie.api.io.ResourceType;
import org.kie.api.runtime.KieSession;
import org.kie.api.runtime.rule.QueryResults;
import org.kie.api.runtime.rule.QueryResultsRow;
import org.kie.api.runtime.rule.Variable;
import org.kie.internal.utils.KieHelper;

import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.IdentityHashMap;
import java.util.Locale;
import java.util.TimeZone;

/**
 * >96-key resize sub-recon graft (RunnerDump pattern, modeled on
 * ExistsDump): runs the scenario's standalone query, prints the row
 * order, then reflects into every reachable node memory and dumps the
 * raw layout of each TupleIndexHashTable — table length, per-bucket
 * chains head->tail, each entry's cached hashCode and fact values.
 * The point: separate the TABLE STATE after AbstractHashTable.resize
 * (bucket assignment + within-chain order = the "chain reversal"
 * question) from the row-EMISSION order, so the >96 model is pinned
 * against ground truth instead of composite black-box fits.
 * Diagnostic only; never part of the gate.
 *
 * Usage: java ... dev.seine.oracle.QueryIndexDump <scenario.json>
 */
public final class QueryIndexDump {

    private static final String PKG = "seine.gen";
    private static final ObjectMapper M = new ObjectMapper();

    public static void main(String[] args) throws Exception {
        Locale.setDefault(new Locale("en", "US"));
        TimeZone.setDefault(TimeZone.getTimeZone("UTC"));
        JsonNode scenario = M.readTree(Files.readString(Path.of(args[0])));
        String drl = "package " + PKG + ";\n"
                + declareBlocks(scenario.path("types")) + "\n" + scenario.path("drl").asText();
        KieBase kbase = new KieHelper().addContent(drl, ResourceType.DRL).build();
        KieSession session = kbase.newKieSession();
        for (JsonNode fact : scenario.path("facts")) {
            session.insert(instantiate(kbase, scenario, fact));
        }
        session.fireAllRules(10_000);

        for (JsonNode q : scenario.path("queries")) {
            Object[] qargs = new Object[q.path("args").size()];
            for (int i = 0; i < qargs.length; i++) {
                JsonNode a = q.path("args").get(i);
                qargs[i] = a.isNull() ? Variable.v : (Object) a.asLong();
            }
            QueryResults res = session.getQueryResults(q.path("call").asText(), qargs);
            StringBuilder sb = new StringBuilder("ROWS ").append(q.path("call").asText()).append(": ");
            for (QueryResultsRow row : res) {
                for (String ident : res.getIdentifiers()) {
                    Object v = row.get(ident);
                    if (v != null && !(v instanceof Variable)) sb.append(v).append(' ');
                }
            }
            System.out.println(sb);
            dumpIndexTables(session);   // state as left after the call
        }
        session.dispose();
    }

    static void dumpIndexTables(KieSession session) {
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
        try {
            Object bm = call1(reteEval, "getNodeMemory", node,
                    Class.forName("org.drools.core.common.MemoryFactory"));
            if (bm != null) {
                for (String side : new String[]{"getRightTupleMemory", "getLeftTupleMemory"}) {
                    Object tm = callOrNull(bm, side);
                    if (tm != null && tm.getClass().getSimpleName().equals("TupleIndexHashTable")) {
                        dumpTable(node, side, tm);
                    }
                }
            }
        } catch (Exception ignore) {
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

    static void dumpTable(Object node, String side, Object tm) throws Exception {
        Object[] table = (Object[]) field(tm, "table");
        Object size = field(tm, "size");
        Object threshold = field(tm, "threshold");
        System.out.println("TABLE node=" + node.getClass().getSimpleName() + call(node, "getId")
                + " side=" + side + " len=" + table.length
                + " size=" + size + " threshold=" + threshold);
        for (int i = 0; i < table.length; i++) {
            if (table[i] == null) continue;
            StringBuilder sb = new StringBuilder("  b").append(i).append(": ");
            for (Object e = table[i]; e != null; e = callOrNull(e, "getNext")) {
                sb.append('[').append("h=").append(e.hashCode()).append(' ');
                for (Object t = callOrNull(e, "getFirst"); t != null; t = callOrNull(t, "getNext")) {
                    Object fh = callOrNull(t, "getFactHandle");
                    if (fh != null) {
                        Object o = call(fh, "getObject");
                        sb.append(short_(o)).append(' ');
                    }
                }
                sb.append("] ");
            }
            System.out.println(sb);
        }
    }

    static Object field(Object o, String name) throws Exception {
        for (Class<?> c = o.getClass(); c != null; c = c.getSuperclass()) {
            try {
                Field f = c.getDeclaredField(name);
                f.setAccessible(true);
                return f.get(o);
            } catch (NoSuchFieldException ignore) {
            }
        }
        throw new NoSuchFieldException(name);
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
        String tname = fact.path("type").asText();
        FactType ft = kbase.getFactType(PKG, tname);
        Object bean = ft.newInstance();
        java.util.Iterator<String> it = fact.path("fields").fieldNames();
        while (it.hasNext()) {
            String fname = it.next();
            JsonNode v = fact.path("fields").path(fname);
            ft.set(bean, fname, v.asLong());
        }
        return bean;
    }
}
