"""Use the unchanged sequential runner for the frozen one-workload repair cohort."""
import run

run.BENCHES = ['regexredux']
if __name__ == '__main__':
    run.main()
