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
        // SEINE_TIME=1: per-scenario wall time on stderr ("TIME <name> <ms>",
        // parse+compile+run+serialize) for tools/bench_oracle.py. stderr-only,
        // env-gated — the NDJSON contract on stdout is untouched.
        boolean timed = System.getenv("SEINE_TIME") != null;
        for (String arg : args) {
            long t0 = timed ? System.nanoTime() : 0L;
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
            if (timed) {
                System.err.println("TIME " + name + " " + (System.nanoTime() - t0) / 1_000_000.0);
            }
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
        try {
            ArrayNode firings = M.createArrayNode();
            final KieSession fsession = session;
            final org.kie.api.event.rule.AgendaEventListener firingListener =
                    new DefaultAgendaEventListener() {
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
            };
            session.addEventListener(firingListener);

            // D-047: the VISIBLE insertion sequence (external + rule
            // inserts, InitialFact filtered) — external actions target
            // facts by index into this list, matching the engine's.
            final java.util.List<FactHandle> inserted = new ArrayList<>();
            // CEP E2 item D: the entry point each NAMED-EP fact was inserted
            // into — update/delete must route through it (session.update/delete
            // on a named-EP handle throws "Invalid Entry Point"). Default/RHS
            // facts are absent → routed through the default EP.
            final java.util.Map<FactHandle, org.kie.api.runtime.rule.EntryPoint> epMap =
                    new java.util.HashMap<>();
            final org.kie.api.event.rule.RuleRuntimeEventListener insertListener =
                    new DefaultRuleRuntimeEventListener() {
                @Override
                public void objectInserted(ObjectInsertedEvent event) {
                    if (!event.getObject().getClass().getSimpleName().equals("InitialFactImpl")) {
                        inserted.add(event.getFactHandle());
                    }
                }
            };
            session.addEventListener(insertListener);

            for (JsonNode fact : scenario.path("facts")) {
                insertFact(session, kbase, scenario, fact, epMap);
            }
            int fired = session.fireAllRules(FIRE_LIMIT);
            if (fired >= FIRE_LIMIT) {
                throw new IllegalStateException("fire limit " + FIRE_LIMIT + " reached (non-terminating?)");
            }
            // Multi-fire epochs (D-046) + external WM actions (D-047):
            // ordered actions, then legacy "facts" inserts, then fire.
            ArrayNode queryOut = M.createArrayNode();
            for (JsonNode epoch : scenario.path("epochs")) {
                for (JsonNode action : epoch.path("actions")) {
                    String op = action.path("op").asText();
                    if (op.equals("insert")) {
                        insertFact(session, kbase, scenario, action, epMap);
                    } else if (op.equals("update")) {
                        int target = action.path("target").asInt();
                        FactHandle fh = inserted.get(target);
                        org.kie.api.runtime.rule.EntryPoint ep =
                                epMap.getOrDefault(fh, session.getEntryPoint("DEFAULT"));
                        Object bean = ep.getObject(fh);
                        FactType ft = kbase.getFactType(PKG, bean.getClass().getSimpleName());
                        java.util.List<String> props = new ArrayList<>();
                        java.util.Iterator<String> it = action.path("fields").fieldNames();
                        while (it.hasNext()) {
                            String fname = it.next();
                            JsonNode v = action.path("fields").path(fname);
                            setTyped(ft, bean, fname, v, scenario);
                            props.add(fname);
                        }
                        // property-masked external update, routed through the
                        // fact's entry point (CEP E2 item D)
                        ep.update(fh, bean, props.toArray(new String[0]));
                    } else if (op.equals("delete")) {
                        int target = action.path("target").asInt();
                        FactHandle fh = inserted.get(target);
                        epMap.getOrDefault(fh, session.getEntryPoint("DEFAULT")).delete(fh);
                    } else if (op.equals("advance")) {
                        // CEP E0: pseudo-clock advance (ms). Expiration
                        // jobs run inside advanceTime (clock set to each
                        // trigger's own fire time); their retractions
                        // propagate at this epoch's fireAllRules.
                        ((org.drools.core.time.SessionPseudoClock) session.getSessionClock())
                                .advanceTime(action.path("ms").asLong(),
                                        java.util.concurrent.TimeUnit.MILLISECONDS);
                    } else if (op.equals("reset")) {
                        // Arc 2 (D-104): in-place session reset —
                        // StatefulKnowledgeSessionImpl.reset() clears WM,
                        // agenda, handle counters, entry points and the
                        // pseudo-clock, KEEPING the KieBase. The runner's
                        // insertion index restarts with it.
                        ((org.drools.kiesession.session.StatefulKnowledgeSessionImpl) session).reset();
                        inserted.clear();
                        epMap.clear();
                        // reset() drops event listeners (measured, rs_r1/r2)
                        // — re-register the runner's observability
                        session.addEventListener(firingListener);
                        session.addEventListener(insertListener);
                    } else {
                        throw new IllegalArgumentException("unknown epoch action op: " + op);
                    }
                }
                for (JsonNode fact : epoch.path("facts")) {
                    insertFact(session, kbase, scenario, fact, epMap);
                }
                fired = session.fireAllRules(FIRE_LIMIT);
                // Arc 5 (D-107): per-epoch query invocation
                if (epoch.has("queries")) {
                    runQueryCalls(kbase, session, epoch.path("queries"), queryOut);
                }
                if (fired >= FIRE_LIMIT) {
                    throw new IllegalStateException("fire limit " + FIRE_LIMIT + " reached (non-terminating?)");
                }
            }

            // Query invocation phase: after all epochs, against final WM.
            // Scenario "queries" = ordered calls {"call": name, "args": [...]},
            // JSON null arg = unbound (Variable.v). Result entry echoes the
            // call and captures identifiers + rows in iteration order.
            runQueryCalls(kbase, session, scenario.path("queries"), queryOut);

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

    /** Generate DRL declare blocks from the ordered type schema.
     *  CEP E0 (D-099): a type-level {"event": {"timestamp": f,
     *  "expires_ms": N}} object renders @role(event) + @timestamp +
     *  optional @expires — absent for the whole certified corpus. */
    static String declareBlocks(JsonNode types) {
        StringBuilder sb = new StringBuilder();
        for (JsonNode type : types) {
            sb.append("declare ").append(type.path("name").asText()).append('\n');
            JsonNode ev = type.path("event");
            if (!ev.isMissingNode()) {
                sb.append("    @role( event )\n");
                if (ev.has("timestamp")) {
                    sb.append("    @timestamp( ").append(ev.path("timestamp").asText()).append(" )\n");
                }
                if (ev.has("duration")) {
                    // CEP E2 item E (recon): @duration(field) makes T an
                    // INTERVAL event occupying [ts, ts+field]. Field-name
                    // reference like @timestamp. Corpus-inert (absent for the
                    // whole certified corpus).
                    sb.append("    @duration( ").append(ev.path("duration").asText()).append(" )\n");
                }
                if (ev.has("expires_ms")) {
                    sb.append("    @expires( ").append(ev.path("expires_ms").asLong()).append("ms )\n");
                }
            }
            for (JsonNode field : type.path("fields")) {
                // @key on every field (D-076): declared-type equality =
                // value-equality over all fields, so Drools TMS equality
                // keys match the engine's D-066 mechanism. Inert for the
                // pre-TMS corpus (identity assert mode; hashing keys on
                // field values) — proven by full-tier re-runs.
                sb.append("    ").append(field.path("name").asText())
                  .append(" : ").append(javaType(field.path("type").asText()))
                  .append(" @key").append('\n');
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

    /** CEP E2 item D: insert a fact into its named entry point (`from
     *  entry-point "X"`) when the fact/action carries "entry_point", else the
     *  DEFAULT entry point. objectInserted still fires (D-047 target list). */
    static void insertFact(KieSession session, KieBase kbase, JsonNode scenario, JsonNode factNode,
                           java.util.Map<FactHandle, org.kie.api.runtime.rule.EntryPoint> epMap)
            throws Exception {
        Object obj = instantiate(kbase, scenario, factNode);
        String ep = factNode.path("entry_point").asText("");
        if (ep.isEmpty()) {
            session.insert(obj);
        } else {
            org.kie.api.runtime.rule.EntryPoint entryPoint = session.getEntryPoint(ep);
            epMap.put(entryPoint.insert(obj), entryPoint);
        }
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
        if (o instanceof java.util.Set<?> s) {
            // D-108: set iteration order is raw HashSet internals —
            // canonicalize SORTED (by rendered JSON) under a distinct
            // type so list order stays significant.
            node.put("type", "SetCollection");
            java.util.List<com.fasterxml.jackson.databind.JsonNode> rendered =
                    new ArrayList<>();
            for (Object e : s) {
                rendered.add(render(kbase, session, e));
            }
            rendered.sort(java.util.Comparator.comparing(Object::toString));
            ArrayNode arr = node.putObject("fields").putArray("value");
            for (com.fasterxml.jackson.databind.JsonNode e : rendered) {
                arr.add(e);
            }
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
        // ?query CEs in rules contribute an Object[] (the query element's
        // row arguments) to the match; raw toString carries an identity
        // hash, so canonicalize with ORDER-significant elements (Q2).
        if (o instanceof Object[] arr0) {
            node.put("type", "QueryArgs");
            ArrayNode arr = node.putObject("fields").putArray("value");
            for (Object e : arr0) {
                arr.add(e == null ? M.nullNode() : render(kbase, session, e));
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

    private static void runQueryCalls(KieBase kbase,
            org.kie.api.runtime.KieSession session,
            JsonNode calls, ArrayNode queryOut) {
        for (JsonNode q : calls) {
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
                        Object v;
                        try {
                            v = row.get(id);
                        } catch (RuntimeException e) {
                            // identifiers local to another or-branch are
                            // absent from this row: row.get throws
                            v = null;
                        }
                        ro.set(id, v == null ? M.nullNode() : render(kbase, session, v));
                    }
                    rows.add(ro);
                }
                queryOut.add(qo);
            }
    }
}
