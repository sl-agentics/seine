package dev.seine.oracle;

import java.io.FileInputStream;
import java.io.InputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;

import org.drools.decisiontable.InputType;
import org.drools.decisiontable.SpreadsheetCompiler;
import org.drools.decisiontable.parser.DefaultRuleSheetListener;
import org.drools.decisiontable.parser.RuleMatrixSheetListener;

/**
 * D-391 decision-table oracle: CSV in, Drools' DRL text out.
 *
 *   TableDump rules  table.csv   SpreadsheetCompiler.compile(stream, InputType.CSV)  (DefaultRuleSheetListener)
 *   TableDump matrix table.csv   the same with a RuleMatrixSheetListener
 *
 *   TableDump batch              stdin lines "rules|matrix<TAB>path"; for each, stdout gets
 *                                "\u0001BEGIN path\n" + output + "\u0001END\n" (one JVM for a whole corpus)
 *
 * A parse error prints "ERROR <exception class>: <message>" (exit 3 in single mode), so the port's error surface is
 * pinned too.
 */
public final class TableDump {
    public static void main(String[] args) throws Exception {
        PrintStream out = new PrintStream(System.out, true, StandardCharsets.UTF_8);
        if (args.length == 1 && "batch".equals(args[0])) {
            java.io.BufferedReader r = new java.io.BufferedReader(new java.io.InputStreamReader(System.in, StandardCharsets.UTF_8));
            String line;
            while ((line = r.readLine()) != null) {
                String[] mp = line.split("\t", 2);
                out.print("\u0001BEGIN " + mp[1] + "\n");
                out.print(compileOne(mp[0], mp[1]));
                out.print("\u0001END\n");
            }
            return;
        }
        if (args.length != 2) {
            System.err.println("usage: TableDump rules|matrix table.csv");
            System.exit(2);
        }
        String res = compileOne(args[0], args[1]);
        out.print(res);
        if (res.startsWith("ERROR ")) {
            System.exit(3);
        }
    }

    static String compileOne(String mode, String path) {
        try (InputStream in = new FileInputStream(path)) {
            SpreadsheetCompiler c = new SpreadsheetCompiler();
            return "matrix".equals(mode)
                    ? c.compile(in, InputType.CSV, new RuleMatrixSheetListener())
                    : c.compile(in, InputType.CSV);
        } catch (Exception e) {
            return "ERROR " + e.getClass().getSimpleName() + ": " + e.getMessage();
        }
    }
}
