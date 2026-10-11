/*
 * The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

 * contributed by Jon Edvardsson
 * added parallel processing to the original
 * program by Anthony Donnefort and Enotus.
 */

import java.io.IOException;
import java.util.LinkedList;
import java.util.List;
import java.util.concurrent.ForkJoinPool;
import java.util.concurrent.RecursiveAction;

public final class revcomp {

    static final ForkJoinPool fjPool = new ForkJoinPool();

    static final byte[] map = new byte[128];

    static {
        String[] mm = {"ACBDGHK\nMNSRUTWVYacbdghkmnsrutwvy",
                       "TGVHCDM\nKNSYAAWBRTGVHCDMKNSYAAWBR"};
        for (int i = 0; i < mm[0].length(); i++)
            map[mm[0].charAt(i)] = (byte) mm[1].charAt(i);
    }

    private static class Reverse extends RecursiveAction {
        private byte[] buf;
        private int begin;
        private int end;

        public Reverse(byte[] buf, int begin, int end) {
            this.buf = buf;
            this.begin = begin;
            this.end = end;
        }

        protected void compute() {
            byte[] buf = this.buf;
            int begin = this.begin;
            int end = this.end;

            while (true) {
                byte bb = buf[begin];
                if (bb == '\n')
                    bb = buf[++begin];
                byte be = buf[end];
                if (be == '\n')
                    be = buf[--end];
                if (begin > end)
                    break;
                buf[begin++] = be;
                buf[end--] = bb;
            }
        }
    }

    public static void main(String[] args) throws IOException, InterruptedException {
        final byte[] buf = new byte[System.in.available()];
        System.in.read(buf);
        List<Reverse> tasks = new LinkedList<Reverse>();

        for (int i = 0; i < buf.length; ) {
            while (buf[i++] != '\n') ;
            int data = i;
            byte b;
            while (i < buf.length && (b = buf[i++]) != '>') {
                buf[i-1] = map[b];
            }
            Reverse task = new Reverse(buf, data, i - 2);
            fjPool.execute(task);
            tasks.add(task);
        }
        for (Reverse task : tasks) {
            task.join();
        }

        System.out.write(buf);
    }
}
