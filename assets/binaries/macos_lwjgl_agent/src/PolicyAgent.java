import java.io.*;
import java.lang.instrument.Instrumentation;
import java.nio.file.*;

public class PolicyAgent {
    public static void premain(String args, Instrumentation inst) {
        System.err.println("[agent] premain running");
        try {
            // Prefer explicit override if present, otherwise use bundled copy
            String override = System.getProperty("policy.lib");
            File lib;
            if (override != null) {
                lib = new File(override);
            } else {
                lib = extractBundled();
            }
            if (lib == null || !lib.exists()) {
                System.err.println("[agent] could not resolve libpolicy.dylib");
                return;
            }
            System.load(lib.getAbsolutePath());
            PolicyJNI.setActivationPolicy();
            System.err.println("[agent] activation policy set");
        } catch (Throwable t) {
            System.err.println("[agent] failed: " + t);
            t.printStackTrace();
        }
    }

    private static File extractBundled() throws IOException {
        try (InputStream in = PolicyAgent.class.getResourceAsStream("/libpolicy.dylib")) {
            if (in == null) return null;
            Path tmp = Files.createTempFile("libpolicy", ".dylib");
            tmp.toFile().deleteOnExit();
            Files.copy(in, tmp, StandardCopyOption.REPLACE_EXISTING);
            return tmp.toFile();
        }
    }
}
