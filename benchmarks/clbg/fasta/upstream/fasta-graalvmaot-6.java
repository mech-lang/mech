/*
 * The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 * 
 * modified by Mehmet D. AKIN
 * modified by Daryl Griffith
 * modified by Mike
 */

import java.io.IOException;
import java.io.OutputStream;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.atomic.AtomicInteger;

public class fasta {
    /** Maximum length of the FASTA sequence lines. */
    private static final int LINE_LENGTH = 60;

    /** Maximum number of FASTA sequence lines that we process at one time. */
    private static final int LINE_COUNT = 1024;

    /** The threads that convert random numbers to nucleotide codes. */
    private static final NucleotideSelector[] WORKERS = new NucleotideSelector[
        Math.max(Runtime.getRuntime().availableProcessors() - 1, 1)
    ];

    private static final AtomicInteger IN = new AtomicInteger();
    private static final AtomicInteger OUT = new AtomicInteger();
    private static final int BUFFERS_IN_PLAY = 6;

    public static void main(String[] args) {
        int n = 1000;

        if (args.length > 0) {
            n = Integer.parseInt(args[0]);
        }
        for (int i = 0; i < WORKERS.length; i++) {
            WORKERS[i] = new NucleotideSelector();
            WORKERS[i].setDaemon(true);
            WORKERS[i].start();
        }
        try (OutputStream writer = System.out) {
            int bufferSize = LINE_COUNT * LINE_LENGTH;

            for (int i = 0; i < BUFFERS_IN_PLAY; i++) {
                lineFillALU(
                    new AluBuffer(LINE_LENGTH, bufferSize, i * bufferSize));
            }
            speciesFillALU(writer, n * 2, ">ONE Homo sapiens alu\n");
            for (int i = 0; i < BUFFERS_IN_PLAY; i++) {
                writeBuffer(writer);
                lineFillRandom(new IubBuffer(LINE_LENGTH, bufferSize));
            }
            speciesFillRandom(writer
                    , n * 3
                    , ">TWO IUB ambiguity codes\n"
                    , true);
            for (int i = 0; i < BUFFERS_IN_PLAY; i++) {
                writeBuffer(writer);
                lineFillRandom(new SapienBuffer(LINE_LENGTH, bufferSize));
            }
            speciesFillRandom(writer
                    , n * 5
                    , ">THREE Homo sapiens frequency\n"
                    , false);
            for (int i = 0; i < BUFFERS_IN_PLAY; i++) {
                writeBuffer(writer);
            }
        } catch (IOException ex) {
            ex.printStackTrace();
        }
     }

    private static void lineFillALU(AbstractBuffer buffer) {
        WORKERS[OUT.incrementAndGet() % WORKERS.length].put(buffer);
    }

    private static void bufferFillALU(OutputStream writer
            , int buffers) throws IOException {
        for (int i = 0; i < buffers; i++) {
            AbstractBuffer buffer =
                WORKERS[IN.incrementAndGet() % WORKERS.length].take();
            writer.write(buffer.nucleotides);
            lineFillALU(buffer);
        }
    }

    private static void speciesFillALU(OutputStream writer, int nChars
            , String name) throws IOException {
        int bufferCount = nChars / (LINE_COUNT * LINE_LENGTH);
        int charsLeftover = nChars % (LINE_COUNT * LINE_LENGTH);

        writer.write(name.getBytes());
        bufferFillALU(writer, bufferCount - BUFFERS_IN_PLAY);
        if (charsLeftover > 0) {
            writeBuffer(writer);
            lineFillALU(new AluBuffer(LINE_LENGTH, charsLeftover,
                nChars - charsLeftover));
        }
    }

    private static void lineFillRandom(StochasticBuffer buffer) {
        buffer.fillRandoms();
        WORKERS[OUT.incrementAndGet() % WORKERS.length].put(buffer);
    }

    private static void bufferFillRandom(OutputStream writer
            , int loops) throws IOException {
        for (int i = 0; i < loops; i++) {
            AbstractBuffer buffer =
                WORKERS[IN.incrementAndGet() % WORKERS.length].take();
            writer.write(buffer.nucleotides);
            lineFillRandom((StochasticBuffer) buffer);
        }
    }

    private static void speciesFillRandom(OutputStream writer
            , int nChars
            , String name
            , boolean isIUB) throws IOException {
        int bufferSize = LINE_COUNT * LINE_LENGTH;
        int bufferCount = nChars / bufferSize;
        int bufferLoops = bufferCount - BUFFERS_IN_PLAY;
        int charsLeftover = nChars - (bufferCount * bufferSize);

        writer.write(name.getBytes());
        bufferFillRandom(writer, bufferLoops);
        if (charsLeftover > 0) {
            writeBuffer(writer);    
            lineFillRandom(isIUB
                ? new IubBuffer(LINE_LENGTH, charsLeftover)
                : new SapienBuffer(LINE_LENGTH, charsLeftover)
            );
        }
    }

    private static void writeBuffer(OutputStream writer) throws IOException {
        writer.write(
            WORKERS[IN.incrementAndGet() % WORKERS.length].take().nucleotides
        );
    }

    private static class NucleotideSelector extends Thread {
        private final BlockingQueue<AbstractBuffer>
            in = new ArrayBlockingQueue<>(BUFFERS_IN_PLAY);
        private final BlockingQueue<AbstractBuffer> 
            out = new ArrayBlockingQueue<>(BUFFERS_IN_PLAY);

        public void put(AbstractBuffer line) {
            try {
                in.put(line);
            } catch (InterruptedException ex) {
                ex.printStackTrace();
            }
        }

        @Override
        public void run() {
            try {
                for (;;) {
                    AbstractBuffer line= in.take();
                    line.selectNucleotides();
                    out.put(line);
                }
            } catch (InterruptedException ex) {
                ex.printStackTrace();
            }
        }

        public AbstractBuffer take() {
            try {
                return out.take();
            } catch (InterruptedException ex) {
                ex.printStackTrace();
            }
            return null;
        }
    }

    private abstract static class AbstractBuffer {
        protected final int LINE_LENGTH;
        protected final int LINE_COUNT;
        protected final byte[] nucleotides;
        protected final int CHARS_LEFTOVER;

        AbstractBuffer(int lineLength, int nChars) {
            LINE_LENGTH = lineLength;
            int outputLineLength = lineLength + 1;
            LINE_COUNT = nChars / lineLength;
            CHARS_LEFTOVER = nChars % lineLength;
            int nucleotidesSize
                = nChars + LINE_COUNT + (CHARS_LEFTOVER == 0 ? 0 : 1);
            int lastNucleotide = nucleotidesSize - 1;

            nucleotides = new byte[nucleotidesSize];
            for (int i = lineLength
                    ; i < lastNucleotide
                    ; i += outputLineLength) {
                nucleotides[i] = '\n';
            }
            nucleotides[nucleotides.length - 1] = '\n';
        }

        abstract void selectNucleotides();
    }

    private static class AluBuffer extends AbstractBuffer {
        private static final String ALU =
            "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"
            + "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"
            + "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"
            + "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"
            + "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"
            + "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
            + "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";
        private static final int ALU_LENGTH = ALU.length();

        private final int MAX_ALU_INDEX = ALU_LENGTH - LINE_LENGTH;
        private final int ALU_ADJUST = LINE_LENGTH - ALU_LENGTH;
        private final int nChars;
        private int charIndex;
        private int nucleotideIndex;
        private final byte[] chars;

        public AluBuffer(int lineLength, int nChars, int offset) {
            super(lineLength, nChars);
            this.nChars = nChars;
            chars = (ALU + ALU.substring(0, LINE_LENGTH)).getBytes();
            charIndex = offset % ALU_LENGTH;
        }

        @Override
        void selectNucleotides() {
            nucleotideIndex = 0;
            for (int i = 0; i < LINE_COUNT; i++) {
                ALUFillLine(LINE_LENGTH);
            }
            if (CHARS_LEFTOVER > 0) {
                ALUFillLine(CHARS_LEFTOVER);
            }
            charIndex += nChars * (BUFFERS_IN_PLAY - 1);
            charIndex %= ALU_LENGTH;
        }

        private void ALUFillLine(int charCount) {
            System.arraycopy(chars
                    , charIndex
                    , nucleotides
                    , nucleotideIndex
                    , charCount);
            charIndex += charIndex < MAX_ALU_INDEX ? charCount : ALU_ADJUST;
            nucleotideIndex += charCount + 1;
        }
    }

    private static abstract class StochasticBuffer extends AbstractBuffer {
        // LCG parameters
        private static final int IM = 139968;
        private static final int IA = 3877;
        private static final int IC = 29573;
        private static final float ONE_OVER_IM = 1f / IM;

        /** The last LCG seed value. */
        private static int last = 42;

        protected final float[] randoms;

        protected StochasticBuffer(int lineLength, int nChars) {
            super(lineLength, nChars);
            randoms = new float[nChars];
        }

        void fillRandoms() {
            for (int i = 0; i < randoms.length; i++) {
                last = (last * IA + IC) % IM;
                randoms[i] = last * ONE_OVER_IM;
            }
        }
    }

    private static final class IubBuffer extends StochasticBuffer {
        private static final byte[] chars = new byte[]{
            'a', 'c', 'g', 't',
            'B', 'D', 'H', 'K',
            'M', 'N', 'R', 'S',
            'V', 'W', 'Y'};
        private static final float[] probs = new float[15];
        static {
            double[] dblProbs = new double[]{
                0.27, 0.12, 0.12, 0.27,
                0.02, 0.02, 0.02, 0.02,
                0.02, 0.02, 0.02, 0.02,
                0.02, 0.02, 0.02};

            double cp = 0;
            for (int i = 0; i < probs.length - 1; i++) {
                cp += dblProbs[i];
                probs[i] = (float) cp;
            }
            probs[probs.length - 1] = 2;
        }

        private final int charsInFullLines;

        IubBuffer(int lineLength, int nChars) {
            super(lineLength, nChars);
            charsInFullLines = (nChars / lineLength) * lineLength;
        }

        @Override
        void selectNucleotides() {
            int i = 0, j = 0;
            for (; i < charsInFullLines; j++) {
                for (int k = 0; k < LINE_LENGTH; k++)
                    nucleotides[j++] = convert(randoms[i++]);
            }
            for (int k = 0; k < CHARS_LEFTOVER; k++)
                nucleotides[j++] = convert(randoms[i++]);
        }

        private static byte convert(float r) {
            /* A binary search is considerably slower than this sequential one.
             * A sequential search of the first four entries followed by a
             * binary search of the rest falls between the two, so is still
             * slower than this.
             *
             * Attempting to use the vectorizedMismatch intrinsic with
             * something like:
             *
             *    Arrays.setAll(temp, i -> probs[i] < r ? 0 : 1);
             *    int m = Arrays.mismatch(temp, zeros);
             *
             * yields much slower results.
             *
             * This code compiles to a sequence of vucomiss/jnbe instructions
             * (on an i7-6500U), which probably executes well speculatively.
             *
             * Explicitly unrolling the loop doesn't improve performance
             * noticeably.
             */
            int m;
            //noinspection StatementWithEmptyBody
            for (m = 0; probs[m] < r; m++) {}
            return chars[m];
        }
    }

    private static final class SapienBuffer extends StochasticBuffer {
        private static final byte[] chars = new byte[]{'a', 'c', 'g', 't'};
        private static final float[] probs = new float[4];
        static {
            double[] dblProbs = new double[]{
                0.3029549426680,
                0.1979883004921,
                0.1975473066391,
                0.3015094502008};

            double cp = 0;
            for (int i = 0; i < probs.length - 1; i++) {
                cp += dblProbs[i];
                probs[i] = (float) cp;
            }
            probs[probs.length - 1] = 2;
        }

        private final int charsInFullLines;

        SapienBuffer(int lineLength, int nChars) {
            super(lineLength, nChars);
            charsInFullLines = (nChars / lineLength) * lineLength;
        }

        @Override
        void selectNucleotides() {
            int i = 0, j = 0;
            for (; i < charsInFullLines; j++) {
                for (int k = 0; k < LINE_LENGTH; k++)
                    nucleotides[j++] = convert(randoms[i++]);
            }
            for (int k = 0; k < CHARS_LEFTOVER; k++)
                nucleotides[j++] = convert(randoms[i++]);
        }

        private static byte convert(float r) {
            int m;
            //noinspection StatementWithEmptyBody
            for (m = 0; probs[m] < r; m++) {}
            return chars[m];
        }
    }
}
