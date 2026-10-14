import java.lang.management.ManagementFactory;
import java.nio.charset.StandardCharsets;
import java.util.Base64;

/** Toolchain metadata only: never loads or invokes a clbg benchmark class. */
public final class RuntimeProbe {
    private static String encoded(String value) {
        return Base64.getEncoder().encodeToString(value.getBytes(StandardCharsets.UTF_8));
    }
    private static void field(String key, String value) {
        System.out.println(key + "=" + encoded(value));
    }
    public static void main(String[] args) {
        if (args.length != 0) throw new IllegalArgumentException("No probe arguments");
        field("JAVA_HOME", System.getProperty("java.home"));
        field("RUNTIME_VERSION", System.getProperty("java.runtime.version"));
        field("VENDOR", System.getProperty("java.vendor"));
        field("OS_ARCH", System.getProperty("os.arch"));
        field("VM_NAME", System.getProperty("java.vm.name"));
        field("VM_INFO", System.getProperty("java.vm.info"));
        field("CLASS_PATH", System.getProperty("java.class.path"));
        field("COMPILER", ManagementFactory.getCompilationMXBean().getName());
        field("PROCESSORS", Integer.toString(Runtime.getRuntime().availableProcessors()));
        field("MAX_HEAP", Long.toString(Runtime.getRuntime().maxMemory()));
        field("FILE_ENCODING", System.getProperty("file.encoding"));
        field("USER_HOME", System.getProperty("user.home"));
        field("JAVA_TMPDIR", System.getProperty("java.io.tmpdir"));
        for (String argument : ManagementFactory.getRuntimeMXBean().getInputArguments())
            field("JVM_ARG", argument);
        for (var collector : ManagementFactory.getGarbageCollectorMXBeans())
            field("COLLECTOR", collector.getName());
    }
}
