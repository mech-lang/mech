module clbg::spectralnorm;
import libc;
import clbg::threads;
import std::math;

fn double denominator(int i, int j)
{
    long sum = (long)i + j;
    return (double)(sum * (sum + 1) / 2 + i + 1);
}

struct Job
{
    double* input;
    double* output;
    int n;
    int first;
    int last;
    bool transpose;
}

fn int run(void* argument)
{
    Job* job = argument;
    for (int i = job.first; i < job.last; i++)
    {
        double value = 0.0;
        for (int j = 0; j < job.n; j++)
        {
            value += job.input[j] / (job.transpose ? denominator(j, i) : denominator(i, j));
        }
        job.output[i] = value;
    }
    return 0;
}

fn void multiply(double* input, double* output, int n, bool transpose)
{
    int count = math::min(8, n);
    Job[8] jobs;
    threads::Handle[8] workers;
    for (int worker = 0; worker < count; worker++)
    {
        jobs[worker] = { input, output, n, n * worker / count, n * (worker + 1) / count, transpose };
        threads::start(&workers[worker], &run, &jobs[worker]);
    }
    for (int worker = 0; worker < count; worker++)
    {
        threads::join(workers[worker]);
    }
}

fn void product(double* input, double* temporary, double* output, int n)
{
    multiply(input, temporary, n, false);
    multiply(temporary, output, n, true);
}

fn int main(int argc, char** argv)
{
    int n = (int)libc::atoll(argv[1]);
    if (n < 1)
    {
        return 2;
    }
    double* u = libc::calloc((usz)n, double::size);
    double* v = libc::calloc((usz)n, double::size);
    double* temporary = libc::calloc((usz)n, double::size);
    if (!u || !v || !temporary)
    {
        return 2;
    }
    for (int i = 0; i < n; i++)
    {
        u[i] = 1.0;
    }
    for (int iteration = 0; iteration < 10; iteration++)
    {
        product(u, temporary, v, n);
        product(v, temporary, u, n);
    }
    double uv = 0.0;
    double vv = 0.0;
    for (int i = 0; i < n; i++)
    {
        uv += u[i] * v[i];
        vv += v[i] * v[i];
    }
    libc::printf("%.9f\n", math::sqrt(uv / vv));
    libc::free(u);
    libc::free(v);
    libc::free(temporary);
    return 0;
}
