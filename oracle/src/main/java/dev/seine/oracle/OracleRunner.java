package dev.seine.oracle;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import org.kie.api.KieBase;
import org.kie.api.definition.type.FactType;
import org.kie.api.event.rule.AfterMatchFiredEvent;
import org.kie.api.event.rule.DefaultAgendaEventListener;
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
                + declareBlocks(scenario.path("types"))
                + "\n"
                + scenario.path("drl").asText();

        KieBase kbase = new KieHelper().addContent(drl, ResourceType.DRL).build();
        KieSession session = kbase.newKieSession();
        try {
            ArrayNode firings = M.createArrayNode();
            session.addEventListener(new DefaultAgendaEventListener() {
                @Override
                public void afterMatchFired(AfterMatchFiredEvent event) {
                    ObjectNode firing = M.createObjectNode();
                    firing.put("rule", event.getMatch().getRule().getName());
                    ArrayNode matches = firing.putArray("matches");
                    for (Object o : event.getMatch().getObjects()) {
                        matches.add(render(kbase, o));
                    }
                    firings.add(firing);
                }
            });

            for (JsonNode fact : scenario.path("facts")) {
                session.insert(instantiate(kbase, scenario, fact));
            }
            int fired = session.fireAllRules(FIRE_LIMIT);
            if (fired >= FIRE_LIMIT) {
                throw new IllegalStateException("fire limit " + FIRE_LIMIT + " reached (non-terminating?)");
            }

            ArrayNode facts = M.createArrayNode();
            for (Object o : session.getObjects()) {
                facts.add(render(kbase, o));
            }
            ObjectNode result = M.createObjectNode();
            result.set("facts", facts);
            result.set("firings", firings);
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
    static ObjectNode render(KieBase kbase, Object o) {
        String simpleName = o.getClass().getSimpleName();
        FactType ft = kbase.getFactType(PKG, simpleName);
        ObjectNode node = M.createObjectNode();
        node.put("type", simpleName);
        ObjectNode fields = node.putObject("fields");
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
