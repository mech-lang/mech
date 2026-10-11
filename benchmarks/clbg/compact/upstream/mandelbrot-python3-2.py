# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by Tupteq
# modified by Simon Descarpentries
# modified for multi-core by Ahmad Syukri

import multiprocessing as mp
import sys

def writer(buff_queue,size):  #i/o is slow, so leave it to only one worker
    from array import array
    buff_pos = 0
    buff = []
    cout = sys.stdout.buffer.write
    while 1:
        try:
            buff.append(buff_queue.get_nowait())
        except:
            if len(buff):
                buff.sort() 
                pos = len(buff) - 1 
                while pos>0:
                    if buff[pos][0] == buff[pos-1][1]:  #connect contiguous segment
                        buff[pos-1] = (buff[pos-1][0],buff[pos][1],buff[pos-1][2]+buff[pos][2])
                        del(buff[pos])
                    pos-=1
                if buff[0][0]==buff_pos:  # write if segment is the next needed one
                    cout(array('B', buff[0][2]).tobytes())
                    buff_pos = buff[0][1]
                    del(buff[0])

            if buff_pos>=size:
                break

def worker(size, task_queue, buff_queue):
    cout = sys.stdout.buffer.write
    fsize = float(size)
    r_size = range(size)
    r_iter = range(50)
    local_abs = abs
    result = []
    y=size  #only to ensure task_head is initialized. not so pretty code
    while 1:
        task = task_queue.get()
        if task is None:
            if len(result):  # push remaining segments
                buff_queue.put((task_head,y+1,result))
            break
        elif task-y-1:
            if len(result):      # try to resume from previous segment unless broken
                buff_queue.put((task_head,y+1,result))
            task_head = task
            result = []
        #got new job
        y = task
        bit_num = 7
        byte_acc = 0
        fy = 2j * y / fsize - 1j
        for x in r_size:
            z = 0j
            c = 2. * x / fsize - 1.5 + fy
     
            for i in r_iter:
                z = z * z + c
                if local_abs(z) >= 2.: break
            else:
                byte_acc += 1 << bit_num
     
            if bit_num == 0:
                result.append(byte_acc)
                bit_num = 7
                byte_acc = 0
            else:
                bit_num -= 1
        if bit_num != 7:
            result.append(byte_acc)

def main():
    size = int(sys.argv[1])
    task_queue = mp.Queue()
    for i in range(size):
        task_queue.put(i)
    buff_queue = mp.Queue()
    num_proc = 64 
    proc = []
    for i in range(num_proc):
        task_queue.put(None)

    worker_args = (size, task_queue, buff_queue)

    for a in range(num_proc):
        p = mp.Process(target=worker, args=worker_args)
        p.start()
        proc.append(p)
    sys.stdout.write("P4\n%d %d\n" % (size, size))
    w = mp.Process(target=writer, args=(buff_queue,size))
    w.start()
    proc.append(w)

    for p in proc:
        p.join()

if __name__=='__main__':
    main()
