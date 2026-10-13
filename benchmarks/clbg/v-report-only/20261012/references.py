"""Independent bounded output oracles; these never produce benchmark timings."""
from collections import Counter
import math
import random
import re

WIDTHS = (1, 2, 3, 4, 6, 12, 18)
QUERIES = ('GGT', 'GGTA', 'GGTATT', 'GGTATTTTAATT', 'GGTATTTTAATTTATAGT')
PATTERNS = (
    'agggtaaa|tttaccct', '[cgt]gggtaaa|tttaccc[acg]',
    'a[act]ggtaaa|tttacc[agt]t', 'ag[act]gtaaa|tttac[agt]ct',
    'agg[act]taaa|ttta[agt]cct', 'aggg[acg]aaa|ttt[cgt]ccct',
    'agggt[cgt]aa|tt[acg]accct', 'agggta[cgt]a|t[acg]taccct',
    'agggtaa[cgt]|[acg]ttaccct',
)
ALU = (
    'GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCT'
    'GAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAA'
    'TTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCT'
    'TGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAG'
    'AGCGAGACTCCGTCTCAAAAA'
)


def nbody(n):
    solar_mass, year = 4.0 * math.pi * math.pi, 365.24
    bodies = [
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, solar_mass],
        [4.84143144246472090, -1.16032004402742839, -.103622044471123109,
         .00166007664274403694 * year, .00769901118419740425 * year,
         -.0000690460016972063023 * year, .000954791938424326609 * solar_mass],
        [8.34336671824457987, 4.12479856412430479, -.403523417114321381,
         -.00276742510726862411 * year, .00499852801234917238 * year,
         .0000230417297573763929 * year, .000285885980666130812 * solar_mass],
        [12.8943695621391310, -15.1111514016986312, -.223307578892655734,
         .00296460137564761618 * year, .00237847173959480950 * year,
         -.0000296589568540237556 * year, .0000436624404335156298 * solar_mass],
        [15.3796971148509165, -25.9193146099879641, .179258772950371181,
         .00268067772490389322 * year, .00162824170038242295 * year,
         -.0000951592254519715870 * year, .0000515138902046611451 * solar_mass],
    ]
    for axis in range(3):
        bodies[0][axis + 3] = -sum(body[axis + 3] * body[6] for body in bodies) / solar_mass

    def energy():
        result = 0.0
        for i, body in enumerate(bodies):
            result += .5 * body[6] * sum(value * value for value in body[3:6])
            for other in bodies[i + 1:]:
                distance = math.sqrt(sum((body[axis] - other[axis]) ** 2 for axis in range(3)))
                result -= body[6] * other[6] / distance
        return result

    before = energy()
    for _ in range(n):
        for i, body in enumerate(bodies):
            for other in bodies[i + 1:]:
                displacement = [body[axis] - other[axis] for axis in range(3)]
                squared = sum(value * value for value in displacement)
                magnitude = .01 / (squared * math.sqrt(squared))
                for axis in range(3):
                    body[axis + 3] -= displacement[axis] * other[6] * magnitude
                    other[axis + 3] += displacement[axis] * body[6] * magnitude
        for body in bodies:
            for axis in range(3):
                body[axis] += .01 * body[axis + 3]
    return f'{before:.9f}\n{energy():.9f}\n'.encode()


def fasta(n):
    seed = 42
    output = bytearray()

    def emit(header, text):
        output.extend(header + b'\n')
        for start in range(0, len(text), 60):
            output.extend(text[start:start + 60])
            output.extend(b'\n')

    emit(b'>ONE Homo sapiens alu', (ALU * ((2 * n + len(ALU) - 1) // len(ALU)))[:2 * n].encode())
    distributions = (
        (b'>TWO IUB ambiguity codes', 3 * n,
         tuple(zip('acgtBDHKMNRSVWY', (.27, .12, .12, .27) + (.02,) * 11))),
        (b'>THREE Homo sapiens frequency', 5 * n,
         tuple(zip('acgt', (.3029549426680, .1979883004921, .1975473066391, .3015094502008)))),
    )
    for header, count, distribution in distributions:
        cdf, total = [], 0.0
        for char, probability in distribution:
            total += probability
            cdf.append((char, total))
        text = bytearray()
        for _ in range(count):
            seed = (seed * 3877 + 29573) % 139968
            draw = seed / 139968.0
            text.append(ord(next(char for char, threshold in cdf if draw < threshold)))
        emit(header, text)
    return bytes(output)


def mandelbrot(n):
    output = bytearray(f'P4\n{n} {n}\n'.encode())
    for y in range(n):
        bits = []
        for x in range(n):
            cr, ci = 2.0 * x / n - 1.5, 2.0 * y / n - 1.0
            zr = zi = 0.0
            inside = 1
            for _ in range(50):
                zr, zi = zr * zr - zi * zi + cr, 2.0 * zr * zi + ci
                if zr * zr + zi * zi > 4.0:
                    inside = 0
                    break
            bits.append(inside)
        bits.extend([0] * ((-n) % 8))
        for start in range(0, len(bits), 8):
            output.append(sum(bit << (7 - offset) for offset, bit in enumerate(bits[start:start + 8])))
    return bytes(output)


def spectralnorm(n):
    def multiply(vector, transpose=False):
        result = []
        for i in range(n):
            total = 0.0
            for j in range(n):
                row, column = (j, i) if transpose else (i, j)
                denominator = (row + column) * (row + column + 1) // 2 + row + 1
                total += vector[j] / denominator
            result.append(total)
        return result

    def ata(vector):
        return multiply(multiply(vector), True)

    u = [1.0] * n
    for _ in range(10):
        v = ata(u)
        u = ata(v)
    return f'{math.sqrt(sum(a * b for a, b in zip(u, v)) / sum(b * b for b in v)):.9f}\n'.encode()


def fannkuchredux(n):
    permutation = list(range(n))
    count = [0] * n
    r, rank, checksum, maximum = n, 0, 0, 0
    while True:
        while r != 1:
            count[r - 1] = r
            r -= 1
        working, flips = permutation.copy(), 0
        while working[0]:
            width = working[0] + 1
            working[:width] = reversed(working[:width])
            flips += 1
        checksum += flips * (-1 if rank % 2 else 1)
        maximum = max(maximum, flips)
        rank += 1
        while True:
            if r == n:
                assert rank == math.factorial(n)
                return f'{checksum}\nPfannkuchen({n}) = {maximum}\n'.encode()
            permutation[:r + 1] = permutation[1:r + 1] + permutation[:1]
            count[r] -= 1
            if count[r] > 0:
                break
            r += 1


def pidigits(n):
    q, r, denominator, k = 1, 0, 1, 1
    digits = []
    while len(digits) < n:
        left = (3 * q + r) // denominator
        right = (4 * q + r) // denominator
        if left == right:
            digits.append(str(left))
            q, r = 10 * q, 10 * (r - left * denominator)
        else:
            factor = 2 * k + 1
            q, r, denominator = q * k, (2 * q + r) * factor, denominator * factor
            k += 1
    output = []
    for start in range(0, n, 10):
        chunk = ''.join(digits[start:start + 10])
        output.append(f'{chunk:<10}\t:{min(start + 10, n)}\n')
    return ''.join(output).encode()


def binarytrees(n):
    maximum = max(6, n)
    output = [f'stretch tree of depth {maximum + 1}\t check: {(1 << (maximum + 2)) - 1}\n']
    for depth in range(4, maximum + 1, 2):
        iterations = 1 << (maximum - depth + 4)
        check = iterations * ((1 << (depth + 1)) - 1)
        output.append(f'{iterations}\t trees of depth {depth}\t check: {check}\n')
    output.append(f'long lived tree of depth {maximum}\t check: {(1 << (maximum + 1)) - 1}\n')
    return ''.join(output).encode()


def records(data):
    result = []
    for line in data.decode('ascii').splitlines():
        if line.startswith('>'):
            result.append([line, []])
        elif result and not line.startswith(';'):
            result[-1][1].append(line)
    return [(header, ''.join(lines)) for header, lines in result]


def revcomp(data):
    complement = str.maketrans(
        'ACGTUMRWSYKVHDBNacgtumrwsykvhdbn',
        'TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN',
    )
    output = []
    for header, text in records(data):
        text = text.translate(complement)[::-1]
        output.append(header + '\n')
        output.extend(text[start:start + 60] + '\n' for start in range(0, len(text), 60))
    return ''.join(output).encode()


def nucleotide_maps(data):
    sequence = next((text.upper() for header, text in records(data) if header.startswith('>THREE')), '')
    return sequence, [Counter(sequence[start:start + width] for start in range(max(0, len(sequence) - width + 1))) for width in WIDTHS]


def knucleotide(data):
    _, maps = nucleotide_maps(data)
    output = []
    for counts in maps[:2]:
        total = sum(counts.values())
        output.extend(f'{key} {100.0 * count / total:.3f}\n' for key, count in sorted(counts.items(), key=lambda item: (-item[1], item[0])))
        output.append('\n')
    output.extend(f'{counts[query]}\t{query}\n' for counts, query in zip(maps[2:], QUERIES))
    return ''.join(output).encode()


def regexredux(data):
    cleaned = re.sub(rb'>[^\n]*\n|\n', b'', data)
    text = cleaned
    for pattern, replacement in (
        (rb'tHa[Nt]', b'<4>'), (rb'aND|caN|Ha[DS]|WaS', b'<3>'),
        (rb'a[NSt]|BY', b'<2>'), (rb'<[^>]*>', b'|'),
        (rb'\|[^|][^|]*\|', b'-'),
    ):
        text = re.sub(pattern, replacement, text)
    output = ''.join(f'{pattern} {len(re.findall(pattern.encode(), cleaned))}\n' for pattern in PATTERNS)
    return (output + f'\n{len(data)}\n{len(cleaned)}\n{len(text)}\n').encode()


def dna_fixtures():
    rng = random.Random(20261012)
    text = ''.join(rng.choice('ACGT') for _ in range(10003)) + QUERIES[-1]
    return (
        ('ties-and-stop', b'>ONE\nACGT\n>THREE\nAAAACCCCGGGGTTTT\n>FOUR\nGGGG\n'),
        ('one-base', b'>THREE\nA\n'), ('empty', b'>THREE\n>FOUR\nGGG\n'),
        ('missing', b'>ONE\nACGT\n'),
        ('crlf-comment-eof', b'>THREE\r\naCgT\r\n;GGTATTTTAATTTATAGT'),
        ('embedded-header-comment', b'>ONE\n;>THREE\nGGG\n>THREE\nACGT\n;>FOUR\nGGTATT\n'),
        ('arbitrary-widths', ('>THREE\n' + '\n'.join(text[start:start + 37].lower() for start in range(0, len(text), 37)) + '\n').encode()),
    )
