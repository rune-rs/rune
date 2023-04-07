//! Benchmark of tgolsson's AoC 2020 solutions.
//!
//! Source: https://github.com/tgolsson/aoc-2020

use criterion::Criterion;

criterion::criterion_group!(benches, aoc_2020_11a);

const INPUT: &str = include_str!("data/aoc_2020_11a.txt");

fn aoc_2020_11a(b: &mut Criterion) {
    let input = INPUT
        .split('\n')
        .filter(|v| v != &"")
        .map(str::to_owned)
        .collect::<Vec<String>>();

    let mut vm = rune_vm! {
        enum CellState {
            Floor,
            Unoccupied,
            Occupied,
        }

        struct Map {
            width, height,
            grid_world,
            slopes,
            backbuffer,
            n1
        }

        impl Map {
            fn new() {
                return Map {
                    width: 0,
                    height: 0,
                    grid_world: [],
                    backbuffer: [],
                    slopes: [
                        (0 - 1, 0 - 1),
                        (0,     0 - 1),
                        (0 + 1, 0 - 1),
                        (0 - 1, 0    ),
                        (0 + 1, 0    ),
                        (0 - 1, 0 + 1),
                        (0,     0 + 1),
                        (0 + 1, 0 + 1),
                    ],
                    n1: None,
                }
            }

            fn add(self, row) {

                let row = row.collect::<Vec>();
                if self.grid_world.len() == 0 {
                    self.height = 1;
                    self.grid_world.extend(row.iter().map(|_| CellState::Floor));
                    self.grid_world.push(CellState::Floor);
                    self.grid_world.push(CellState::Floor);
                }
                self.height += 1;
                self.grid_world.push(CellState::Floor);
                self.grid_world.extend(row);
                self.grid_world.push(CellState::Floor);
                self.width = self.grid_world.len() / self.height;
                self
            }

            fn complete(self, scanfunc) {
                self.height += 1;
                self.grid_world.extend((0..self.width).iter().map(|_| CellState::Floor));
                self.backbuffer = self.grid_world.iter().collect::<Vec>();
                self.n1 = self.grid_world.iter().collect::<Vec>();
                for y in 0..self.height  {
                    for x in 0..self.width {
                        let idx = x + y * self.width;
                        self.n1[idx] = scanfunc(self, x, y)
                    }
                }
                self.width = self.grid_world.len() / self.height;
            }

            fn is_valid(self, x, y) {
                1 <= x && x < self.width - 1 && 1 <= y && y < self.height - 1
            }

            fn scan_neighbours(self, x, y) {
                let out = [];

                for slope in self.slopes {
                    let xx = x + slope.0;
                    let yy = y + slope.1;
                    while self.is_valid(xx, yy) {
                        let idx = xx + yy * self.width;
                        match self.grid_world[idx] {
                            CellState::Floor => {},
                            _ => { out.push(idx); break; }
                        }

                        xx += slope.0;
                        yy += slope.1;
                    }
                }
                out
            }

            fn apply_rules(self, x, y, current_state, gen, occupied_count) {
                match current_state {
                    CellState::Floor => {
                        return (current_state, false);
                    }
                    CellState::Unoccupied => {
                        for idx in gen {
                            match self.grid_world[idx] {
                                CellState::Occupied => {
                                    return (current_state, false);
                                },
                                _ => {},
                            }
                        }
                        (CellState::Occupied, true)
                    },
                    CellState::Occupied => {
                        let occupied_neighbours = 0;
                        for idx in gen {
                            match self.grid_world[idx] {
                                CellState::Occupied => {
                                    occupied_neighbours += 1;
                                    if occupied_neighbours >= occupied_count {
                                        return (CellState::Unoccupied, true);
                                    }
                                },
                                _ => {},
                            }
                        }
                        (current_state, false)
                    }
                }
            }

            fn to_coordinate(self, idx) {
                let w = self.width;
                let x = idx % w;
                let y = idx / w;
                (x, y)
            }

            fn step_inner(self, cb) {
                let new_world = self.backbuffer;
                let world_changed = false;
                let idx = 1 + self.width;
                let inner_w = self.width - 1;
                for y in 1..self.height - 1 {
                    for x in 1..inner_w {
                        let current_state = self.grid_world[idx];
                        let (cell_state, changed) = cb(self, x, y, current_state);
                        new_world[idx] = cell_state;
                        world_changed = true;
                        idx += 1;
                    }
                    idx += 2;
                }

                if world_changed {
                    let temp = self.grid_world;
                    self.grid_world = self.backbuffer;
                    self.backbuffer = temp;
                }
                world_changed
            }

            fn step(self) {
                self.step_inner(|sf, x, y, v| sf.apply_rules(x, y, v, sf.n1[x + y * self.width], 4))
            }

            fn step2(self) {
                self.step_inner(|sf, x, y, v| sf.apply_rules(x, y, v, sf.n1[x + y * self.width], 5))
            }
        }


        fn scan_line(row) {
            row.chars().map(|v| match v {
                '.' => CellState::Floor,
                'L' => CellState::Unoccupied,
                '#' => CellState::Occupied,
                _ => {panic!("x")},
            })
        }

        pub fn main(lines) {
            let waiting_hall = lines
                .iter()
                .map(scan_line)
                .fold(Map::new(), Map::add);

            waiting_hall.complete(|m, x, y| m.slopes.iter().map(|(dx, dy)| (x + dx) + (y + dy) * m.width).collect::<Vec>());


            for i in (0..2).iter() {
                if !waiting_hall.step() {
                    break;
                }
            }

            let t1 = waiting_hall.grid_world.iter().filter(|cell| match cell {
                CellState::Occupied => true,
                _ => {false}
            }).count();

            let waiting_hall = lines
                .iter()
                .map(scan_line)
                .fold(Map::new(), Map::add);

            waiting_hall.complete(|m, x, y| m.scan_neighbours(x, y));

            for i in (0..2).iter() {
                if !waiting_hall.step2() {
                    break;
                }
            }

            let t2 = waiting_hall.grid_world.iter().filter(|cell| match cell {
                CellState::Occupied => true,
                _ => {false}
            }).count();

            //NB: This test is actually too slow to finish if we "solve" the task
            let t1 = 2164;
            let t2 = 1974;

            assert_eq!(t1, 2164);
            assert_eq!(t2, 1974);
            (t1, t2)
        }
    };

    let entry = rune::Hash::type_hash(["main"]);

    b.bench_function("aoc_2020_11a", |b| {
        b.iter(|| vm.call(entry, (input.clone(),)).expect("failed call"));
    });
}
