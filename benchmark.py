#!/usr/bin/env python3
# BENCHMARK.py
#   by Lut99
#
# Created:
#   31 Jan 2025, 10:37:03
# Last edited:
#   31 Jan 2025, 11:12:51
# Auto updated?
#   Yes
#
# Description:
#   Script to benchmark the simulation of the agents for examples described in
#   the paper.
#   
#   Concretely, this will be:
#    - `section6-3-1`
#    - `section6-3-2`
#    - `section6-3-3`
#    - `section6-3-4`
#    - `section6-3-5`
#

import argparse
import math
import os
import shlex
import subprocess
import sys
import time
from typing import List


##### CONSTANTS #####
EXAMPLES = ["section6-3-1", "section6-3-2", "section6-3-3", "section6-3-4", "section6-3-5"]

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))





##### HELPER FUNCTIONS #####
def precompile(ex: str, cargo: List[str], target: str):
    """
        Precompiles one of the examples.

        # Arguments
        - `ex`: The example to precompile.
        - `cargo`: The command to call `cargo` with.
        - `target`: The target path.

        # Errors
        This function handles exceptions internally, but quits the script when so.
    """

    # Ensure we're working from the script dir
    os.chdir(SCRIPT_DIR)

    cmd = cargo + ["build", "--release", "--target-dir", target, "--features", "dataplane,log,serde,slick", "--example", ex]
    handle = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if (code := handle.wait()) != 0:
        stdout = handle.stdout.read().decode("utf-8")
        stderr = handle.stderr.read().decode("utf-8")
        print(f"ERROR: Command {cmd} failed with non-zero exit code {code}\n\nstdout:\n{'-' * 80}\n{stdout}\n{'-' * 80}\n\nstderr:\n{'-' * 80}\n{stderr}\n{'-' * 80}\n", file=sys.stderr)
        sys.exit(1)

    # Else, done

def benchmark(ex: str, target: str) -> float:
    """
        Benchmarks one of the examples once.

        # Arguments
        - `ex`: The example to benchmark.
        - `target`: The target path.

        # Errors
        This function handles exceptions internally, but quits the script when so.
    """

    # Ensure we're working from the script dir
    os.chdir(SCRIPT_DIR)
    cmd = os.path.join(target, "release", "examples", ex)

    # Benchmark it
    start = time.time()
    handle = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    code = handle.wait()
    taken = time.time() - start
    if (code := handle.wait()) != 0:
        stdout = handle.stdout.read().decode("utf-8")
        stderr = handle.stderr.read().decode("utf-8")
        print(f"ERROR: Command {cmd} failed with non-zero exit code {code}\n\nstdout:\n{'-' * 80}\n{stdout}\n{'-' * 80}\n\nstderr:\n{'-' * 80}\n{stderr}\n{'-' * 80}\n", file=sys.stderr)
        sys.exit(1)

    # Else, done
    return taken



##### ENTRYPOINT #####
def main(examples: List[str], times: int, cargo: str, target: str) -> int:
    """
        Entrypoint of the script.

        # Arguments
        - `examples`: The list of examples to benchmark.
        - `times`: The number of times to benchmark and take the average of.
        - `cargo`: The command to call `cargo` with.
        - `target`: The path of the `target` build output folder.

        # Returns
        The exit code of the script. `0` is good, anything else is bad.
    """

    # Parse the cargo command
    cargo = shlex.split(cargo)

    # Simply pre-compile all examples first
    print("> Compiling examples")
    for i, ex in enumerate(examples):
        print(f">>> Example '{ex}' ({i + 1}/{len(examples)})...")
        precompile(ex, cargo, target)

    # Now benchmark them
    print("> Benchmarking examples")
    for i, ex in enumerate(examples):
        print(f">>> Example '{ex}' ({i + 1}/{len(examples)})...")
        taken_ms = []
        for t in range(times):
            print(f">>>>> Run {t + 1}/{times}... ", end=""); sys.stdout.flush()
            taken_ms.append(benchmark(ex, target) * 1000.0)
            print(f" {taken_ms[-1]:.2f}ms"); sys.stdout.flush()
        mean = sum(taken_ms) / times
        print(f">>>>> Mean: {mean:.2f}ms, standard deviation: {sum([math.pow(ms - mean, 2) for ms in taken_ms]) / times:.2f}ms")

    return 0


# Actual entrypoint
if __name__ == "__main__":
    parser = argparse.ArgumentParser(formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument("-e", "--examples", type=str, nargs='*', default=EXAMPLES, help="The examples to benchmark. Any option in the Rust project is accepted.")
    parser.add_argument("-t", "--times", type=int, default=10, help="The number of times to benchmark each example. These will be averaged into a single result.")
    parser.add_argument("-C", "--cargo", type=str, default="cargo", help="The command to call whenever we want to call `cargo`.")
    parser.add_argument("-T", "--target", type=str, default=os.path.join(SCRIPT_DIR, "target"), help="The path to the target folder where we will put compiled artifacts in.")

    args = parser.parse_args()

    exit(main(args.examples, args.times, args.cargo, args.target))
