package dev.seine.oracle;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import org.kie.api.KieBase;
import org.kie.api.definition.type.FactType;
import org.kie.api.event.rule.AfterMatchFiredEvent;
import org.kie.api.event.rule.DefaultAgendaEventListener;
import org.kie.api.event.rule.DefaultRuleRuntimeEventListener;
import org.kie.api.event.rule.ObjectInsertedEvent;
import org.kie.api.runtime.rule.FactHandle;
import org.kie.api.runtime.rule.QueryResults;
import org.kie.api.runtime.rule.QueryResultsRow;
import org.kie.api.runtime.rule.Variable;
import org.kie.api.io.ResourceType;
import org.kie.api.runtime.KieSession;
import org.kie.internal.utils.KieHelper;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.TimeZone;

/**
 * Reference runner: executes scenario JSON files through real Drools
 * (pinned version in pom.xml) and emits one NDJSON line per scenario:
 *   {"scenario": <name>, "result": {"facts": [...], "firings": [...]}}
 * or {"scenario": <name>, "error": "..."} on failure.
 *
 * Fact types are NOT compiled Java classes: they are generated as DRL
 * `declare` blocks in package seine.gen from the scenario's type schema,
 * and instantiated through the FactType API, so the oracle is fully
 * data-driven.
 */
public final class OracleRunner {

    private static final String PKG = "seine.gen";
    private static final int FIRE_LIMIT = 100_000;
    private static final ObjectMapper M = new ObjectMapper();

    public static void main(String[] args) throws Exception {
        Locale.setDefault(new Locale("en", "US"));
        TimeZone.setDefault(TimeZone.getTimeZone("UTC"));
        if (args.length == 0) {
            System.err.println("usage: OracleRunner <scenario.json>...");
            System.exit(2);
        }
        for (String arg : args) {
            ObjectNode out = M.createObjectNode();
            String name = arg;
            try {
                JsonNode scenario = M.readTree(Files.readString(Path.of(arg)));
                name = scenario.path("name").asText(arg);
                out.put("scenario", name);
                out.set("result", run(scenario));
            } catch (Throwable t) {
                out.put("scenario", name);
                out.put("error", t.getClass().getSimpleName() + ": " + String.valueOf(t.getMessage()));
            }
            System.out.println(M.writeValueAsString(out));
        }
    }

    static JsonNode run(JsonNode scenario) throws Exception {
        String drl = "package " + PKG + ";\n"
                + "import java.util.List;\n"
                + "import java.util.ArrayList;\n"
                + "import java.util.Collection;\n"
                + declareBlocks(scenario.path("types"))
                + "\n"
                + scenario.path("drl").asText();

        KieBase kbase = new KieHelper().addContent(drl, ResourceType.DRL).build();
        KieSession session = kbase.newKieSession();
        try {
            ArrayNode firings = M.createArrayNode();
            final KieSession fsession = session;
            session.addEventListener(new DefaultAgendaEventListener() {
                @Override
                public void afterMatchFired(AfterMatchFiredEvent event) {
                    ObjectNode firing = M.createObjectNode();
                    firing.put("rule", event.getMatch().getRule().getName());
                    ArrayNode matches = firing.putArray("matches");
                    for (Object o : event.getMatch().getObjects()) {
                        matches.add(render(kbase, fsession, o));
                    }
                    firings.add(firing);
                }
            });

            // D-047: the VISIBLE insertion sequence (external + rule
            // inserts, InitialFact filtered) — external actions target
            // facts by index into this list, matching the engine's.
            final java.util.List<FactHandle> inserted = new ArrayList<>();
            session.addEventListener(new DefaultRuleRuntimeEventListener() {
                @Override
                public void objectInserted(ObjectInsertedEvent event) {
                    if (!event.getObject().getClass().getSimpleName().equals("InitialFactImpl")) {
                        inserted.add(event.getFactHandle());
                    }
                }
            });

            for (JsonNode fact : scenario.path("facts")) {
                session.insert(instantiate(kbase, scenario, fact));
            }
            int fired = session.fireAllRules(FIRE_LIMIT);
            if (fired >= FIRE_LIMIT) {
                throw new IllegalStateException("fire limit " + FIRE_LIMIT + " reached (non-terminating?)");
            }
            // Multi-fire epochs (D-046) + external WM actions (D-047):
            // ordered actions, then legacy "facts" inserts, then fire.
            for (JsonNode epoch : scenario.path("epochs")) {
                for (JsonNode action : epoch.path("actions")) {
                    String op = action.path("op").asText();
                    if (op.equals("insert")) {
                        session.insert(instantiate(kbase, scenario, action));
                    } else if (op.equals("update")) {
                        FactHandle fh = inserted.get(action.path("target").asInt());
                        Object bean = session.getObject(fh);
                        FactType ft = kbase.getFactType(PKG, bean.getClass().getSimpleName());
                        java.util.List<String> props = new ArrayList<>();
                        java.util.Iterator<String> it = action.path("fields").fieldNames();
                        while (it.hasNext()) {
                            String fname = it.next();
                            JsonNode v = action.path("fields").path(fname);
                            setTyped(ft, bean, fname, v, scenario);
                            props.add(fname);
                        }
                        // property-masked external update: the engine
                        // mirrors with the changed-fields mask
                        session.update(fh, bean, props.toArray(new String[0]));
                    } else if (op.equals("delete")) {
                        FactHandle fh = inserted.get(action.path("target").asInt());
                        session.delete(fh);
                    } else {
                        throw new IllegalArgumentException("unknown epoch action op: " + op);
                    }
                }
                for (JsonNode fact : epoch.path("facts")) {
                    session.insert(instantiate(kbase, scenario, fact));
                }
                fired = session.fireAllRules(FIRE_LIMIT);
                if (fired >= FIRE_LIMIT) {
                    throw new IllegalStateException("fire limit " + FIRE_LIMIT + " reached (non-terminating?)");
                }
            }

            // Query invocation phase: after all epochs, against final WM.
            // Scenario "queries" = ordered calls {"call": name, "args": [...]},
            // JSON null arg = unbound (Variable.v). Result entry echoes the
            // call and captures identifiers + rows in iteration order.
            ArrayNode queryOut = M.createArrayNode();
            for (JsonNode q : scenario.path("queries")) {
                String qname = q.path("call").asText();
                List<Object> qargs = new ArrayList<>();
                for (JsonNode a : q.path("args")) {
                    if (a.isNull()) qargs.add(Variable.v);
                    else if (a.isTextual()) qargs.add(a.asText());
                    else if (a.isBoolean()) qargs.add(a.asBoolean());
                    else if (a.isFloatingPointNumber()) qargs.add(a.asDouble());
                    else qargs.add(a.asLong());
                }
                QueryResults res = session.getQueryResults(qname, qargs.toArray());
                ObjectNode qo = M.createObjectNode();
                qo.put("call", qname);
                qo.set("args", q.path("args").deepCopy());
                ArrayNode ids = qo.putArray("identifiers");
                for (String id : res.getIdentifiers()) ids.add(id);
                ArrayNode rows = qo.putArray("rows");
                for (QueryResultsRow row : res) {
                    ObjectNode ro = M.createObjectNode();
                    for (String id : res.getIdentifiers()) {
                        Object v = row.get(id);
                        ro.set(id, v == null ? M.nullNode() : render(kbase, session, v));
                    }
                    rows.add(ro);
                }
                queryOut.add(qo);
            }

            ArrayNode facts = M.createArrayNode();
            for (Object o : session.getObjects()) {
                facts.add(render(kbase, session, o));
            }
            ObjectNode result = M.createObjectNode();
            result.set("facts", facts);
            result.set("firings", firings);
            result.set("queries", queryOut);
            return result;
        } finally {
            session.dispose();
        }
    }

    /** Generate DRL declare blocks from the ordered type schema. */
    static String declareBlocks(JsonNode types) {
        StringBuilder sb = new StringBuilder();
        for (JsonNode type : types) {
            sb.append("declare ").append(type.path("name").asText()).append('\n');
            for (JsonNode field : type.path("fields")) {
                sb.append("    ").append(field.path("name").asText())
                  .append(" : ").append(javaType(field.path("type").asText())).append('\n');
            }
            sb.append("end\n");
        }
        return sb.toString();
    }

    static String javaType(String t) {
        switch (t) {
            case "i64": return "long";
            case "f64": return "double";
            case "String": return "String";
            case "bool": return "boolean";
            default: throw new IllegalArgumentException("unknown field type: " + t);
        }
    }

    /** Set one bean field with the schema-declared type (D-047). */
    static void setTyped(FactType ft, Object bean, String fname, JsonNode v, JsonNode scenario) {
        String typeName = bean.getClass().getSimpleName();
        String declared = null;
        for (JsonNode t : scenario.path("types")) {
            if (t.path("name").asText().equals(typeName)) {
                for (JsonNode f : t.path("fields")) {
                    if (f.path("name").asText().equals(fname)) {
                        declared = f.path("type").asText();
                    }
                }
            }
        }
        if (declared == null) throw new IllegalArgumentException(typeName + " has no field " + fname);
        switch (declared) {
            case "i64": ft.set(bean, fname, v.asLong()); break;
            case "f64": ft.set(bean, fname, v.asDouble()); break;
            case "String": ft.set(bean, fname, v.asText()); break;
            case "bool": ft.set(bean, fname, v.asBoolean()); break;
        }
    }

    static Object instantiate(KieBase kbase, JsonNode scenario, JsonNode fact) throws Exception {
        String typeName = fact.path("type").asText();
        FactType ft = kbase.getFactType(PKG, typeName);
        if (ft == null) throw new IllegalArgumentException("unknown fact type: " + typeName);
        JsonNode schema = null;
        for (JsonNode t : scenario.path("types")) {
            if (t.path("name").asText().equals(typeName)) schema = t;
        }
        if (schema == null) throw new IllegalArgumentException("no schema for type: " + typeName);
        Object bean = ft.newInstance();
        for (JsonNode field : schema.path("fields")) {
            String fname = field.path("name").asText();
            JsonNode v = fact.path("fields").path(fname);
            switch (field.path("type").asText()) {
                case "i64": ft.set(bean, fname, v.asLong()); break;
                case "f64": ft.set(bean, fname, v.asDouble()); break;
                case "String": ft.set(bean, fname, v.asText()); break;
                case "bool": ft.set(bean, fname, v.asBoolean()); break;
            }
        }
        return bean;
    }

    /** Canonical rendering: {"type": T, "fields": {sorted by FactType field order is fine;
     *  the comparator canonicalizes semantically anyway}} */
    static ObjectNode render(KieBase kbase, KieSession session, Object o) {
        String simpleName = o.getClass().getSimpleName();
        ObjectNode node = M.createObjectNode();
        // Rules with a leading not/exists CE match on InitialFactImpl;
        // its toString carries an identity hash, so canonicalize (D-031).
        if (simpleName.equals("InitialFactImpl")) {
            node.put("type", "InitialFact");
            node.putObject("fields");
            return node;
        }
        // Accumulate results are Numbers; collect results are Collections
        // (D-038). Query rows also bind String field values. Canonicalize
        // all scalars — collection classes normalize to "Collection" with
        // an ORDER-SIGNIFICANT element array.
        if (o instanceof String s) {
            node.put("type", "String");
            node.putObject("fields").put("value", s);
            return node;
        }
        if (o instanceof Number || o instanceof Boolean) {
            node.put("type", simpleName);
            ObjectNode f = node.putObject("fields");
            if (o instanceof Long l) f.put("value", l);
            else if (o instanceof Integer i) f.put("value", (long) (int) i);
            else if (o instanceof Double d) f.put("value", d);
            else if (o instanceof Float fl) f.put("value", (double) (float) fl);
            else if (o instanceof Boolean b) f.put("value", b);
            else f.put("value", o.toString());
            return node;
        }
        if (o instanceof java.util.Collection<?> c) {
            node.put("type", "Collection");
            ArrayNode arr = node.putObject("fields").putArray("value");
            for (Object e : c) {
                arr.add(render(kbase, session, e));
            }
            return node;
        }
        FactType ft = kbase.getFactType(PKG, simpleName);
        node.put("type", simpleName);
        ObjectNode fields = node.putObject("fields");
        if (System.getenv("SEINE_HANDLES") != null) {
            org.kie.api.runtime.rule.FactHandle h = session.getFactHandle(o);
            fields.put("__h", h == null ? "?" : h.toExternalForm());
        }
        if (ft != null) {
            ft.getAsMap(o).forEach((k, v) -> {
                if (v instanceof Long l) fields.put(k, l);
                else if (v instanceof Integer i) fields.put(k, (long) i);
                else if (v instanceof Double d) fields.put(k, d);
                else if (v instanceof Boolean b) fields.put(k, b);
                else fields.put(k, String.valueOf(v));
            });
        } else {
            fields.put("_unrenderable", o.toString());
        }
        return node;
    }
}
