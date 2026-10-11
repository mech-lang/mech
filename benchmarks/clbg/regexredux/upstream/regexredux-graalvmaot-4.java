/*
   The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Piotr Tarsa
*/

import jextract_pcre2.pcre2_h;
import org.graalvm.nativeimage.hosted.Feature;
import org.graalvm.nativeimage.hosted.RuntimeForeignAccess;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.MemorySegment;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Map.Entry;
import java.util.concurrent.Callable;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.function.Consumer;
import java.util.function.Function;

import static java.lang.foreign.ValueLayout.*;

public class regexredux {
    private static final ExecutorService EXECUTOR_SERVICE =
            Executors.newFixedThreadPool(Runtime.getRuntime().availableProcessors());

    private static final Arena GLOBAL_ARENA = Arena.global();

    public static void main(String[] args) throws Exception {
        final byte[] rawInput = System.in.readAllBytes();
        final int initialLength = rawInput.length;

        final var sequence = GLOBAL_ARENA.allocate(initialLength);
        final int sequenceLength = withArena(arena -> {
            var rawInputBuffer = arena.allocateFrom(JAVA_BYTE, rawInput);
            var compiledPattern = compilePattern(">.*\\n|\\n");
            return substitute(compiledPattern, rawInputBuffer, initialLength,
                    pcre2_h.NULL(), sequence, initialLength, "");
        });

        var magicRegExpsCount = EXECUTOR_SERVICE.submit(() -> {
            final Map<String, String> iub = new LinkedHashMap<>();
            iub.put("tHa[Nt]", "<4>");
            iub.put("aND|caN|Ha[DS]|WaS", "<3>");
            iub.put("a[NSt]|BY", "<2>");
            iub.put("<[^>]*>", "|");
            iub.put("\\|[^|][^|]*\\|", "-");

            return withArena(arena -> {
                var currentLength = sequenceLength;
                var bufLength = currentLength * 3 / 2;
                var buf1 = arena.allocate(bufLength);
                var buf2 = arena.allocate(bufLength);
                MemorySegment.copy(sequence, 0, buf1, 0, sequenceLength);
                var flip = false;

                for (Entry<String, String> entry : iub.entrySet()) {
                    var pattern = entry.getKey();
                    var replacement = entry.getValue();

                    var compiledPattern = compilePattern(pattern);
                    currentLength = substitute(compiledPattern,
                            flip ? buf2 : buf1, currentLength,
                            pcre2_h.NULL(),
                            flip ? buf1 : buf2, bufLength,
                            replacement);
                    flip = !flip;
                }
                return currentLength;
            });
        });

        var variants = Arrays.asList("agggtaaa|tttaccct",
                "[cgt]gggtaaa|tttaccc[acg]",
                "a[act]ggtaaa|tttacc[agt]t",
                "ag[act]gtaaa|tttac[agt]ct",
                "agg[act]taaa|ttta[agt]cct",
                "aggg[acg]aaa|ttt[cgt]ccct",
                "agggt[cgt]aa|tt[acg]accct",
                "agggta[cgt]a|t[acg]taccct",
                "agggtaa[cgt]|[acg]ttaccct");

        var tasks = variants.stream().map(variant -> (Callable<String>) () -> {
            var compiledPattern = compilePattern(variant);
            var oVectorSize = 100;
            var matchData = pcre2_h
                    .pcre2_match_data_create_8(oVectorSize, pcre2_h.NULL());
            var oVectorPtr = pcre2_h
                    .pcre2_get_ovector_pointer_8(matchData)
                    .reinterpret(16 * oVectorSize);
            oVectorPtr.setAtIndex(JAVA_LONG, 1, 0);
            long count = 0;
            var result = 1;
            while ((result = pcre2_h.pcre2_jit_match_8(compiledPattern,
                    sequence, sequenceLength,
                    oVectorPtr.getAtIndex(JAVA_LONG, 2L * result - 1), 0,
                    matchData, pcre2_h.NULL())) > 0) count += result;
            if (result != pcre2_h.PCRE2_ERROR_NOMATCH()) {
                showPcre2ErrorIfAny("jit match", result);
            }
            return variant + " " + count;
        }).toList();

        for (var result : EXECUTOR_SERVICE.invokeAll(tasks)) {
            System.out.println(result.get());
        }

        System.out.println();
        System.out.println(initialLength);
        System.out.println(sequenceLength);
        System.out.println(magicRegExpsCount.get());

        EXECUTOR_SERVICE.shutdown();
    }

    private static int substitute(
            MemorySegment compiledPattern,
            MemorySegment inputBuffer, int inputLength,
            MemorySegment matchContext,
            MemorySegment outputBuffer, int outputBufferLength,
            String replacement) {
        return withArena(arena -> {
            var replacementBytes =
                    replacement.getBytes(StandardCharsets.US_ASCII);
            var replacementBuffer = arena.allocateFrom(
                    JAVA_BYTE, replacementBytes);
            var outputLengthHolder = arena.allocate(JAVA_LONG);
            outputLengthHolder.setAtIndex(JAVA_LONG, 0, outputBufferLength);
            var options = pcre2_h.PCRE2_SUBSTITUTE_GLOBAL() |
                    pcre2_h.PCRE2_NO_UTF_CHECK();
            var substitutionResult = pcre2_h.pcre2_substitute_8(
                    compiledPattern,
                    inputBuffer, inputLength,
                    0, options, pcre2_h.NULL(),
                    matchContext,
                    replacementBuffer, replacementBytes.length,
                    outputBuffer, outputLengthHolder);
            showPcre2ErrorIfAny("substitutionResult", substitutionResult);
            return substitutionResult < 0 ?
                    0 : outputLengthHolder.getAtIndex(JAVA_INT, 0);
        });
    }

    private static MemorySegment compilePattern(String pattern) {
        return withArena(arena -> {
            var patternBytes = pattern.getBytes(StandardCharsets.US_ASCII);
            var patternLength = patternBytes.length;
            var bufPattern = arena.allocateFrom(JAVA_BYTE, patternBytes);
            var bufErrorCode = arena.allocate(pcre2_h.int64_t);
            var bufErrorOffset = arena.allocate(pcre2_h.int64_t);
            var compiledPattern = pcre2_h.pcre2_compile_8(
                    bufPattern, patternLength, 0,
                    bufErrorCode, bufErrorOffset, pcre2_h.NULL());
            if (compiledPattern.equals(pcre2_h.NULL())) {
                showPcre2Error("pcre2_compile_8 failed at offset " +
                                bufErrorOffset.getAtIndex(JAVA_INT, 0),
                        bufErrorCode.getAtIndex(JAVA_INT, 0));
            }
            var jitCompileResult = pcre2_h.pcre2_jit_compile_8(
                    compiledPattern, pcre2_h.PCRE2_JIT_COMPLETE());
            showPcre2ErrorIfAny("pcre_2jit_compile_8", jitCompileResult);
            return compiledPattern;
        });
    }

    private static void showPcre2ErrorIfAny(
            String description, int resultOrErrorCode) {
        if (resultOrErrorCode < 0) {
            showPcre2Error(description, resultOrErrorCode);
        }
    }

    private static void showPcre2Error(String description, int errorCode) {
        withArena(arena -> {
            var bufSize = 1000;
            var buf = arena.allocate(bufSize);
            var errorMsgLength = pcre2_h.pcre2_get_error_message_8(
                    errorCode, buf, bufSize);
            if (errorMsgLength >= 0) {
                var errorMsgBytes = new byte[errorMsgLength];
                buf.asByteBuffer().get(errorMsgBytes);
                var errorMsg = new String(errorMsgBytes, 0, errorMsgLength,
                        StandardCharsets.US_ASCII);
                new Exception(description + " " + errorCode +
                        " = " + errorMsg).printStackTrace(System.out);
            } else {
                new Exception(description +
                        " Error during getting error message: " +
                        errorMsgLength).printStackTrace(System.out);
            }
        });
    }

    private static void withArena(Consumer<Arena> body) {
        try (var arena = Arena.ofConfined()) {
            body.accept(arena);
        }
    }

    private static <T> T withArena(Function<Arena, T> body) {
        try (var arena = Arena.ofConfined()) {
            return body.apply(arena);
        }
    }
}

class ForeignRegistrationFeature implements Feature {
    public void duringSetup(DuringSetupAccess access) {
        RuntimeForeignAccess.registerForDowncall(FunctionDescriptor.of(
                pcre2_h.C_POINTER,
                pcre2_h.C_POINTER
        ));
        RuntimeForeignAccess.registerForDowncall(FunctionDescriptor.of(
                pcre2_h.C_POINTER,
                pcre2_h.C_INT,
                pcre2_h.C_POINTER
        ));
        RuntimeForeignAccess.registerForDowncall(FunctionDescriptor.of(
                pcre2_h.C_INT,
                pcre2_h.C_POINTER,
                pcre2_h.C_INT
        ));
        RuntimeForeignAccess.registerForDowncall(FunctionDescriptor.of(
                pcre2_h.C_INT,
                pcre2_h.C_INT,
                pcre2_h.C_POINTER,
                pcre2_h.C_LONG
        ));
        RuntimeForeignAccess.registerForDowncall(FunctionDescriptor.of(
                pcre2_h.C_POINTER,
                pcre2_h.C_POINTER,
                pcre2_h.C_LONG,
                pcre2_h.C_INT,
                pcre2_h.C_POINTER,
                pcre2_h.C_POINTER,
                pcre2_h.C_POINTER
        ));
        RuntimeForeignAccess.registerForDowncall(FunctionDescriptor.of(
                pcre2_h.C_INT,
                pcre2_h.C_POINTER,
                pcre2_h.C_POINTER,
                pcre2_h.C_LONG,
                pcre2_h.C_LONG,
                pcre2_h.C_INT,
                pcre2_h.C_POINTER,
                pcre2_h.C_POINTER
        ));
        RuntimeForeignAccess.registerForDowncall(FunctionDescriptor.of(
                pcre2_h.C_INT,
                pcre2_h.C_POINTER,
                pcre2_h.C_POINTER,
                pcre2_h.C_LONG,
                pcre2_h.C_LONG,
                pcre2_h.C_INT,
                pcre2_h.C_POINTER,
                pcre2_h.C_POINTER,
                pcre2_h.C_POINTER,
                pcre2_h.C_LONG,
                pcre2_h.C_POINTER,
                pcre2_h.C_POINTER
        ));
    }
}
