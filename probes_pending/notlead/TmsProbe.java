import org.kie.api.KieBase;
import org.kie.api.definition.type.FactType;
import org.kie.api.event.rule.*;
import org.kie.api.io.ResourceType;
import org.kie.api.runtime.KieSession;
import org.kie.api.runtime.rule.FactHandle;
import org.kie.internal.utils.KieHelper;

public class TmsProbe {
    static String pkg = "dev.seine.gen";
    public static void main(String[] args) throws Exception {
        String drl = "package " + pkg + ";\n" +
            "declare P v : long end\n" +
            "declare B2 n : long end\n" +
            "declare K2 n : long end\n" +
            "rule W when $k : K2($n : n) then insertLogical(new B2($n)); end\n" +
            "rule R when not B2(n >= 1) P($v : v) then end\n";
        KieBase kbase = new KieHelper().addContent(drl, ResourceType.DRL).build();
        KieSession s = kbase.newKieSession();
        s.addEventListener(new RuleRuntimeEventListener() {
            public void objectInserted(ObjectInsertedEvent e) { System.out.println("  WM-INS  " + e.getObject()); }
            public void objectUpdated(ObjectUpdatedEvent e)  { System.out.println("  WM-UPD  " + e.getObject()); }
            public void objectDeleted(ObjectDeletedEvent e)  { System.out.println("  WM-DEL  " + e.getOldObject()); }
        });
        s.addEventListener(new DefaultAgendaEventListener() {
            public void matchCreated(MatchCreatedEvent e)   { System.out.println("  MATCH+  " + e.getMatch().getRule().getName() + " " + e.getMatch().getObjects()); }
            public void matchCancelled(MatchCancelledEvent e){ System.out.println("  MATCH-  " + e.getMatch().getRule().getName() + " (" + e.getCause() + ")"); }
            public void beforeMatchFired(BeforeMatchFiredEvent e) { System.out.println("  FIRE    " + e.getMatch().getRule().getName() + " " + e.getMatch().getObjects()); }
        });
        FactType tP = kbase.getFactType(pkg, "P");
        FactType tK = kbase.getFactType(pkg, "K2");
        Object p0 = tP.newInstance(); tP.set(p0, "v", 1L);
        Object k2 = tK.newInstance(); tK.set(k2, "n", 5L);
        System.out.println("== e0: insert P(1), K2(5); fire");
        s.insert(p0);
        FactHandle hk = s.insert(k2);
        s.fireAllRules();
        System.out.println("== e1: update K2 n=7; insert P(2); fire");
        tK.set(k2, "n", 7L); s.update(hk, k2);
        Object p1 = tP.newInstance(); tP.set(p1, "v", 2L); s.insert(p1);
        s.fireAllRules();
        System.out.println("== e2: update K2 n=9; insert P(4); fire");
        tK.set(k2, "n", 9L); s.update(hk, k2);
        Object p2 = tP.newInstance(); tP.set(p2, "v", 4L); s.insert(p2);
        s.fireAllRules();
        System.out.println("== e3: update K2 n=0; fire");
        tK.set(k2, "n", 0L); s.update(hk, k2);
        s.fireAllRules();
        System.out.println("== done");
    }
}
