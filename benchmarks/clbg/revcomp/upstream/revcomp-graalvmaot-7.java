/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

 * contributed by Sassa NF
 * fork-join + NIO
 *
 * very little left from original contribution by
 * Jon Edvardsson which in turn is based on
 * the original program by Anthony Donnefort and Enotus.
 */

import java.io.*;
import java.nio.*;
import java.nio.channels.*;
import java.util.ArrayList;
import java.util.concurrent.*;

public class revcomp extends RecursiveTask<ArrayList<revcomp>> {
  private static final byte[] map = new byte[128];
  private static final ByteBuffer bytes;
  private static final Semaphore processed = new Semaphore(0);
  private static final int THRESHOLD = 1024*1024;
  private static final ForkJoinPool jobs = new ForkJoinPool();

  private static int tails = 0;

  static {
    ByteBuffer b = null;
    try {
      b = ByteBuffer.allocateDirect(System.in.available());
    } catch(Exception ioe) {
    }
    bytes = b;
    String[] mm = { "ACBDGHK\nMNSRUTWVYacbdghkmnsrutwvy",
                    "TGVHCDM\nKNSYAAWBRTGVHCDMKNSYAAWBR" };
    for (int i = 0; i < mm[0].length(); i++)
      map[mm[0].charAt(i)] = (byte) mm[1].charAt(i);
  }

  private static void reverse(ByteBuffer bytes, int f, int t) {
    if ( f >= t ) return;
    for(;;) {
      int b = bytes.get(f);
      if (b == '\n') b = bytes.get(++f);
      int e = bytes.get(--t);
      if (e == '\n') e = bytes.get(--t);

      if (f > t) break;
      bytes.put(f++, map[e]);
      bytes.put(t, map[b]);
    }
  }

  private static void reverse(ByteBuffer[] bbs) {
    int head = 0;
    int tail = bbs.length;
    if ( tail == 0 ) return;
    ByteBuffer bh = bbs[head++];
    ByteBuffer bt = bbs[--tail];
    int f = bh.position();
    int t = bt.limit()-1;
    while( bh != bt ) {
      if ( f == bh.limit() ) {
        bh = bbs[head++];
        f = bh.position();
        continue;
      }

      if ( t < bt.position() ) {
        bt = bbs[--tail];
        t = bt.limit()-1;
        continue;
      }

      int b = bh.get(f);
      if (b == '\n' ) {
        f++;
        continue;
      }

      int e = bt.get(t);
      if (e == '\n' ) {
        t--;
        continue;
      }

       bh.put(f++, map[e]);
       bt.put(t--, map[b]);
    }

    reverse(bh, f, t+1);
  }

  public static void main(String [] args) throws Exception {
    long t0 = System.nanoTime();
    FileChannel in = new FileInputStream(FileDescriptor.in).getChannel();
    while(bytes.hasRemaining()) {
      in.read(bytes);
    }
    bytes.flip();

    revcomp t = new revcomp(bytes);
    jobs.execute(t);

    ArrayList<revcomp> bs = t.flatten();
    ArrayList<ByteBuffer> head = new ArrayList<ByteBuffer>(bs.size()*2);
    ArrayList<ByteBuffer> tail = new ArrayList<ByteBuffer>(bs.size()*2);
    for( revcomp b: bs ) b.foldl( head, tail );

    reverse(tail.toArray(new ByteBuffer[0]));

    head.addAll(tail);
    ByteBuffer[] toWrite = head.toArray(new ByteBuffer[0]);

    processed.acquire(tails);

    FileChannel out = new FileOutputStream(FileDescriptor.out).getChannel();

    long sz=0;
    while(( sz += out.write(toWrite) ) < bytes.capacity());
    out.force(true);

    System.err.println( ( System.nanoTime() - t0 ) / 1e9 );
  }

  ByteBuffer[] bufs;
  ByteBuffer buf;
  int firstt;
  int lastf;

  public revcomp(ByteBuffer b) {
    buf = b;
  }

  public revcomp(ByteBuffer[] bs) {
    bufs = bs;
  }

  protected ArrayList<revcomp> compute() {
    if (bufs != null) {
      reverse(bufs);
      processed.release(1);
      return null;
    }

    ArrayList<revcomp> al = new ArrayList<revcomp>();

    while( buf.remaining() > THRESHOLD ) {
      int next = (buf.position() + buf.limit()) / 2;
         // assuming well-formed input, the buffer must contain
         // at least one CRLF in THRESHOLD bytes
      while( buf.get(next) != '\n' ) next--; 
      ByteBuffer b = buf.slice();
      b.limit(next+1-buf.position());
      buf.position(next+1);
      revcomp t = new revcomp(b);
      jobs.execute(t);
      al.add(t);
    }

    int f = buf.position();
    int t = f;
    while(t < buf.limit() && buf.get(t) != '>') t++;
    lastf = f;
    firstt = t; 
       // this is where the first entry ends; 
       // it must go before the last entry of the previous job
    for(;;) {
         // we don't reverse the first entry, 
         // and we don't reverse the last entry
      while(t < buf.limit() && buf.get(t) != '\n') t++;
      if ( t == buf.limit() ) break;
      f = t+1;
      lastf = f;

      while(t < buf.limit() && buf.get(t) != '>') t++;
      if ( t == buf.limit() ) break;

      reverse( buf, f, t );
    }

    return al;
  }

  public ArrayList<revcomp> flatten() throws InterruptedException, ExecutionException {
    ArrayList<revcomp> pre = get();
    ArrayList<revcomp> r = pre.isEmpty() ? pre: new ArrayList<revcomp>();
    for( revcomp p: pre ) r.addAll( p.flatten() );
    r.add(this);
    return r;
  }

  public void foldl(ArrayList<ByteBuffer> head, ArrayList<ByteBuffer> tail) {
    if ( lastf <= firstt ) { 
         // all in one chunk - add all to head
      tail.add(buf);
      return;
    }
    if (firstt > buf.position()) {
      ByteBuffer first = buf.slice();
      first.limit(firstt - buf.position());

      tail.add(first);
    }
    jobs.execute(new revcomp(tail.toArray(new ByteBuffer[0])));
    tails++;
    head.addAll(tail);
    head.add(buf);

    tail.clear();

    ByteBuffer last = buf.slice();
    last.position(lastf - buf.position());
    buf.limit(lastf);
    tail.add(last);

    buf.position(firstt);
  }
}
