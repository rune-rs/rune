mod benchmarks;

criterion::criterion_main! {
    benchmarks::aoc_2020_1a::benches,
    benchmarks::aoc_2020_1b::benches,
    benchmarks::aoc_2020_11a::benches,
    benchmarks::aoc_2020_19b::benches,
    benchmarks::brainfuck::benches,
    benchmarks::fib::benches,
}
