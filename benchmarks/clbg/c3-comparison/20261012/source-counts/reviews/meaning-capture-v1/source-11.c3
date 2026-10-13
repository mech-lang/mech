module clbg::mandelbrot;
import libc;
import std::math;
import clbg::threads;

fn void rows(char* image, int n, int stride, int first, int last)
{
    double scale = 2.0 / (double)n;
    for (int y = first; y < last; y++)
    {
        double ci = (double)y * scale - 1.0;
        for (int x = 0; x < n; x++)
        {
            double cr = (double)x * scale - 1.5;
            double zr = 0.0;
            double zi = 0.0;
            double tr = 0.0;
            double ti = 0.0;
            for (int iteration = 0; iteration < 50; iteration++)
            {
                zi = (zr + zr) * zi + ci;
                zr = tr - ti + cr;
                tr = zr * zr;
                ti = zi * zi;
                if (tr + ti > 4.0)
                {
                    break;
                }
            }
            if (tr + ti <= 4.0)
            {
                image[y * stride + x / 8] |= (char)(128 >> (x % 8));
            }
        }
    }
}

struct Job
{
    char* image;
    int n;
    int stride;
    int first;
    int last;
}

fn int run(void* argument)
{
    Job* job = argument;
    rows(job.image, job.n, job.stride, job.first, job.last);
    return 0;
}

fn int main(int argc, char** argv)
{
    int n = (int)libc::atoll(argv[1]);
    if (n < 1)
    {
        return 2;
    }
    int stride = (n + 7) / 8;
    char* image = libc::calloc((usz)n, (usz)stride);
    if (!image)
    {
        return 2;
    }
    int count = math::min(8, n);
    Job[8] jobs;
    threads::Handle[8] workers;
    for (int worker = 0; worker < count; worker++)
    {
        jobs[worker] = { image, n, stride, n * worker / count, n * (worker + 1) / count };
        threads::start(&workers[worker], &run, &jobs[worker]);
    }
    for (int worker = 0; worker < count; worker++)
    {
        threads::join(workers[worker]);
    }
    libc::printf("P4\n%d %d\n", n, n);
    if (libc::fwrite(image, 1, (usz)n * (usz)stride, libc::stdout()) != (usz)n * (usz)stride)
    {
        return 2;
    }
    libc::free(image);
    return 0;
}
