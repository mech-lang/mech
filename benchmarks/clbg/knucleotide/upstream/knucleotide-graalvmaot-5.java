/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by Daryl Griffith 
 */

import java.io.Closeable;
import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.Map.Entry;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ForkJoinPool;
import java.util.concurrent.ForkJoinTask;
import java.util.concurrent.Future;

public class knucleotide extends ForkJoinTask {

    static final long start = System.nanoTime();
    static final ForkJoinPool pool = new ForkJoinPool(Runtime.getRuntime().availableProcessors());
    static final Map<Key, Value> MAP = new ConcurrentHashMap<>();
    static final int[] SEQUENCES1 = {2, 1};
    static final int[] SEQUENCES2 = {18, 12, 6, 4, 3};
    static final String[] SPICIFIC_SEQUENCES = new String[]{"GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"};
    static final int LINE_LENGTH = 60;
    static final int EOF = -1;
    static byte[] nucleotides;
    Object rawResult;
    final int sequence;

    public static void main(String[] args) {
        {
            byte[] temp = new byte[LINE_LENGTH];
            byte[] buffer = new byte[125_000_000];
            byte[] species = ">TH".getBytes();
            int n;
            int i;

            try (LineInputStream in = new LineInputStream(System.in)) {
outer:
                for (;;) {
                    n = in.readLine(temp);
                    if (n == EOF) {
                        return;
                    }
                    if (n != LINE_LENGTH) {
                        for (i = 0; i < species.length; i++) {
                            if (temp[i] != species[i]) {
                                continue outer;
                            }
                        }
                        break;
                    }
                }
                i = 0;
                for (;;) {
                    n = in.readLine(temp);
                    if (n == EOF) {
                        break;
                    }
                    for (int j = 0; j < n; i++, j++) {
                        buffer[i] = translate(temp[j]);
                    }
                }
                if (i == buffer.length) {
                    nucleotides = buffer;
                } else {
                    nucleotides = new byte[i];
                    System.arraycopy(buffer, 0, nucleotides, 0, i);
                }
            } catch (IOException e) {

            }
        }
        countSequences(SEQUENCES1);
        {
            List<Entry<Key, Value>> sequence1 = new ArrayList<>();
            List<Entry<Key, Value>> sequence2 = new ArrayList<>();

            for (Entry<Key, Value> entry : MAP.entrySet()) {
                switch (Long.numberOfLeadingZeros(entry.getKey().key)) {
                    case 61:
                        sequence1.add(entry);
                        break;
                    case 59:
                        sequence2.add(entry);
                }
            }
            printSequence(sequence1);
            printSequence(sequence2);
        }
        countSequences(SEQUENCES2);
        {
            Key key = new Key();
            for (String sequence : SPICIFIC_SEQUENCES) {
                key.setHash(sequence);
                System.out.print(MAP.get(key).count);
                System.out.print('\t');
                System.out.println(sequence);
            }
        }
    }
        
    static byte translate(byte b) {
        return (byte) ((b >> 1) & 3);
    }

    static void countSequences(int[] sequences) {
        Future[] futures = new Future[sequences.length];
        int i = 0;

        for (int sequence : sequences) {
            futures[i] = pool.submit(new knucleotide(sequence));
            i++;
        }
        for (Future future: futures) {
            try {
                future.get();
            } catch (InterruptedException | ExecutionException ex) {
            }
        }
    }

    static void updateHashtable(int sequence) {
        int sequenceTop = nucleotides.length - sequence + 1;
        Key key = new Key();
        Value value;
        
        for (int i = 0; i < sequenceTop; i++) {
            key.setHash(i, sequence);
            value = MAP.get(key);
            if (value == null) {
                value = new Value();
                value.count = 1;
                MAP.put(key, value);
                key = new Key();
            } else {
                value.count++;
            }
        }
    }

    static void printSequence(List<Entry<Key, Value>> sequence) {
        int sum = 0;

        Collections.sort(sequence, new Comparator<Entry<Key, Value>>() {

            @Override
            public int compare(Entry<Key, Value> entry1, Entry<Key, Value> entry2) {
                if (entry2.getValue().count != entry1.getValue().count) {
                    return entry2.getValue().count - entry1.getValue().count;
                }
                return entry1.getKey().toString().compareTo(entry2.getKey().toString());
            }
        });
        for (Entry<Key, Value> entry : sequence) {
            sum += entry.getValue().count;
        }
        for (Entry<Key, Value> entry : sequence) {
            System.out.format("%s %.3f\n", entry.getKey(), entry.getValue().count * 100f / sum);
        }
        System.out.println();
    }

    public knucleotide(int sequence) {
        this.sequence = sequence;
    }

    @Override
    public Object getRawResult() {
        return rawResult;
    }

    @Override
    protected void setRawResult(Object value) {
        rawResult = value;
    }

    @Override
    protected boolean exec() {
        updateHashtable(sequence);
        setRawResult(new Object());
        return true;
    }
    static class LineInputStream implements Closeable {

        private static final int LF = 10;
        private final ByteBuffer buffer = ByteBuffer.allocate(8192);
        private final InputStream in;

        public LineInputStream(InputStream in) {
            this.in = in;
            buffer.limit(buffer.position());
        }

        public int readLine(byte[] b) throws IOException {
            for (int end = buffer.position(); end < buffer.limit(); end++) {
                if (buffer.get(end) == LF) {
                    if (end - buffer.position() == LINE_LENGTH) {
                        buffer.get(b);
                        buffer.position(buffer.position() + 1);
                        return LINE_LENGTH;
                    } else {
                        int size = end - buffer.position();

                        buffer.get(b, 0, size);
                        buffer.position(buffer.position() + 1);
                        return size;
                    }
                }
            }
            buffer.compact();
            int n = in.read(buffer.array(), buffer.position(), buffer.remaining());

            if (n == EOF) {
                buffer.flip();
                if (buffer.hasRemaining()) {
                    int size = buffer.remaining();

                    buffer.get(b, 0, size);
                    return size;
                } else {
                    return EOF;
                }
            } else {
                buffer.position(buffer.position() + n);
                buffer.flip();
            }
            for (int end = buffer.position(); end < buffer.limit(); end++) {
                if (buffer.get(end) == LF) {
                    if (end - buffer.position() == LINE_LENGTH) {
                        buffer.get(b);
                        buffer.position(buffer.position() + 1);
                        return LINE_LENGTH;
                    } else {
                        int size = end - buffer.position();

                        buffer.get(b, 0, size);
                        buffer.position(buffer.position() + 1);
                        return size;
                    }
                }
            }
            return EOF;
        }

        @Override
        public void close() throws IOException {
            in.close();
        }
    }

    static class Key {

        long key;

        void setHash(int offset, int length) {
            key = 1;
            for (int i = offset + length - 1; i >= offset; i--) {
                key = (key << 2) | nucleotides[i];
            }
        }

        void setHash(String species) {
            key = 1;
            for (int i = species.length() - 1; i >= 0; i--) {
                key = (key << 2) | translate((byte) species.charAt(i));
            }
        }

        @Override
        public int hashCode() {
            return (int) key;
        }

        @Override
        public boolean equals(Object obj) {
            final Key other = (Key) obj;

            return key == other.key;
        }

        @Override
        public String toString() {
            char[] name = new char[(63 - Long.numberOfLeadingZeros(key)) / 2];
            long temp = key;

            for (int i = 0; temp > 1; temp >>= 2, i++) {
                name[i] = (char) (((temp & 3) << 1) | 'A');
                if (name[i] == 'E') {
                    name[i] = 'T';
                }
            }
            return new String(name);
        }
    }

    static class Value {

        int count;
    }
}
