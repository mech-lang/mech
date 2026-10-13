module clbg::fannkuchredux;
import libc;
import std::math;
import clbg::threads;

struct Job
{
    int n;
    long first;
    long last;
    long checksum;
    int maximum;
}

fn int run(void* argument)
{
    Job* job = argument;
    long[13] factorial;
    factorial[0] = 1;
    for (int width = 1; width <= job.n; width++)
    {
        factorial[width] = factorial[width - 1] * width;
    }
    int[12] permutation;
    int[12] rotations;
    for (int i = 0; i < job.n; i++)
    {
        permutation[i] = i;
    }
    for (int width = job.n; width >= 2; width--)
    {
        int shift = (int)((job.first / factorial[width - 1]) % width);
        rotations[width - 1] = shift;
        for (int turn = 0; turn < shift; turn++)
        {
            int first = permutation[0];
            for (int j = 0; j < width - 1; j++)
            {
                permutation[j] = permutation[j + 1];
            }
            permutation[width - 1] = first;
        }
    }
    long checksum = 0;
    int maximum = 0;
    for (long rank = job.first; rank < job.last; rank++)
    {
        int[12] work = permutation;
        int flips = 0;
        while (work[0] != 0)
        {
            int left = 0;
            int right = work[0];
            while (left < right)
            {
                int temporary = work[left];
                work[left] = work[right];
                work[right] = temporary;
                left++;
                right--;
            }
            flips++;
        }
        checksum += (long)((rank & 1) == 0 ? flips : -flips);
        maximum = math::max(maximum, flips);
        for (int width = 2; width <= job.n; width++)
        {
            int first = permutation[0];
            for (int j = 0; j < width - 1; j++)
            {
                permutation[j] = permutation[j + 1];
            }
            permutation[width - 1] = first;
            rotations[width - 1]++;
            if (rotations[width - 1] < width)
            {
                break;
            }
            rotations[width - 1] = 0;
        }
    }
    job.checksum = checksum;
    job.maximum = maximum;
    return 0;
}

fn int main(int argc, char** argv)
{
    int n = (int)libc::atoll(argv[1]);
    if (n < 1 || n > 12)
    {
        return 2;
    }
    long total = 1;
    for (int width = 1; width <= n; width++)
    {
        total *= width;
    }
    int count = (int)math::min(8L, total);
    Job[8] jobs;
    threads::Handle[8] workers;
    for (int worker = 0; worker < count; worker++)
    {
        jobs[worker] = { n, total * worker / count, total * (long)(worker + 1) / count, 0, 0 };
        threads::start(&workers[worker], &run, &jobs[worker]);
    }
    for (int worker = 0; worker < count; worker++)
    {
        threads::join(workers[worker]);
    }
    long checksum = 0;
    int maximum = 0;
    for (int worker = 0; worker < count; worker++)
    {
        checksum += jobs[worker].checksum;
        maximum = math::max(maximum, jobs[worker].maximum);
    }
    libc::printf("%lld\nPfannkuchen(%d) = %d\n", checksum, n, maximum);
    return 0;
}
