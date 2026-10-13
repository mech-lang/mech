module clbg::fannkuchredux;
import libc;
import std::math;

fn int main(int argc, char** argv)
{
    int n = (int)libc::atoll(argv[1]);
    if (n < 1 || n > 12)
    {
        return 2;
    }
    int[12] permutation;
    int[12] rotations;
    long total = 1;
    for (int i = 0; i < n; i++)
    {
        permutation[i] = i;
        total *= i + 1;
    }
    long checksum = 0;
    int maximum = 0;
    for (long rank = 0; rank < total; rank++)
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
        checksum += (rank & 1) == 0 ? flips : -flips;
        maximum = math::max(maximum, flips);
        for (int width = 2; width <= n; width++)
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
    libc::printf("%lld\nPfannkuchen(%d) = %d\n", checksum, n, maximum);
    return 0;
}
