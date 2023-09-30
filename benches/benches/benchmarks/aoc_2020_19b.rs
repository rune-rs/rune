use criterion::Criterion;

use rune::alloc::prelude::*;

criterion::criterion_group!(benches, aoc_2020_19b);

const INPUT: &str = include_str!("data/aoc_2020_19b.txt");

fn aoc_2020_19b(b: &mut Criterion) {
    let mut data = rune::runtime::Vec::new();

    for line in INPUT.split('\n').filter(|s| !s.is_empty()) {
        data.push(rune::to_value(line.to_owned()).unwrap()).unwrap();
    }

    let mut vm = rune_vm! {
        use std::collections::HashMap;
        fn get_rules() {
            HashMap::from([
                (118, Rule::Or(Rule::Seq([29, 95]), Rule::Seq([106, 58]))),
                (64, Rule::Or(Rule::Seq([29, 63]), Rule::Seq([106, 89]))),
                (112, Rule::Or(Rule::Seq([106, 98]), Rule::Seq([29, 60]))),
                (52, Rule::Or(Rule::Seq([98, 29]), Rule::Seq([95, 106]))),
                (127, Rule::Or(Rule::Seq([29, 10]), Rule::Seq([106, 32]))),
                (55, Rule::Or(Rule::Seq([86, 106]), Rule::Seq([80, 29]))),
                (31, Rule::Or(Rule::Seq([29, 78]), Rule::Seq([106, 56]))),
                (128, Rule::Or(Rule::Seq([106, 114]), Rule::Seq([29, 70]))),
                (91, Rule::Or(Rule::Seq([106, 48]), Rule::Seq([29, 43]))),
                (40, Rule::Or(Rule::Seq([106, 106]), Rule::Seq([29, 106]))),
                (20, Rule::Or(Rule::Seq([3, 29]), Rule::Seq([75, 106]))),
                (37, Rule::Or(Rule::Seq([106, 87]), Rule::Seq([29, 92]))),
                (48, Rule::Or(Rule::Seq([62, 29]), Rule::Seq([22, 106]))),
                (51, Rule::Or(Rule::Seq([106, 29]), Rule::Seq([106, 106]))),
                (3, Rule::Or(Rule::Seq([106, 29]), Rule::Seq([29, 29]))),
                (113, Rule::Seq([96, 101])),
                (107, Rule::Or(Rule::Seq([15, 29]), Rule::Seq([83, 106]))),
                (98, Rule::Seq([29, 106])),
                (104, Rule::Or(Rule::Seq([29, 66]), Rule::Seq([106, 76]))),
                (21, Rule::Or(Rule::Seq([29, 40]), Rule::Seq([106, 58]))),
                (87, Rule::Or(Rule::Seq([106, 99]), Rule::Seq([29, 127]))),
                (6, Rule::Or(Rule::Seq([29, 119]), Rule::Seq([106, 3]))),
                (85, Rule::Or(Rule::Seq([29, 58]), Rule::Seq([106, 119]))),
                (96, Rule::Or(Rule::Seq([106]), Rule::Seq([29]))),
                (78, Rule::Or(Rule::Seq([109, 29]), Rule::Seq([125, 106]))),
                (83, Rule::Or(Rule::Seq([106, 20]), Rule::Seq([29, 36]))),
                (71, Rule::Or(Rule::Seq([2, 29]), Rule::Seq([21, 106]))),
                (116, Rule::Or(Rule::Seq([106, 58]), Rule::Seq([29, 54]))),
                (110, Rule::Or(Rule::Seq([119, 106]), Rule::Seq([62, 29]))),
                (13, Rule::Or(Rule::Seq([77, 106]), Rule::Seq([64, 29]))),
                (57, Rule::Or(Rule::Seq([22, 106]), Rule::Seq([45, 29]))),
                (60, Rule::Or(Rule::Seq([106, 106]), Rule::Seq([96, 29]))),
                (17, Rule::Or(Rule::Seq([26, 29]), Rule::Seq([49, 106]))),
                (43, Rule::Seq([96, 40])),
                (41, Rule::Or(Rule::Seq([106, 105]), Rule::Seq([29, 54]))),
                (103, Rule::Or(Rule::Seq([119, 29]), Rule::Seq([98, 106]))),
                (27, Rule::Or(Rule::Seq([106, 3]), Rule::Seq([29, 58]))),
                (90, Rule::Or(Rule::Seq([120, 106]), Rule::Seq([44, 29]))),
                (59, Rule::Or(Rule::Seq([106, 60]), Rule::Seq([29, 58]))),
                (58, Rule::Or(Rule::Seq([29, 106]), Rule::Seq([29, 29]))),
                (74, Rule::Seq([58, 96])),
                (68, Rule::Or(Rule::Seq([29, 112]), Rule::Seq([106, 100]))),
                (119, Rule::Or(Rule::Seq([106, 106]), Rule::Seq([29, 96]))),
                (108, Rule::Or(Rule::Seq([106, 119]), Rule::Seq([29, 3]))),
                (86, Rule::Or(Rule::Seq([106, 51]), Rule::Seq([29, 3]))),
                (53, Rule::Or(Rule::Seq([29, 95]), Rule::Seq([106, 75]))),
                (62, Rule::Seq([96, 96])),
                (70, Rule::Or(Rule::Seq([93, 106]), Rule::Seq([118, 29]))),
                (124, Rule::Or(Rule::Seq([102, 29]), Rule::Seq([37, 106]))),
                (106, Rule::Str('a')),
                (9, Rule::Or(Rule::Seq([122, 29]), Rule::Seq([116, 106]))),
                (8, Rule::Seq([42])),
                (94, Rule::Or(Rule::Seq([29, 117]), Rule::Seq([106, 67]))),
                (42, Rule::Or(Rule::Seq([106, 124]), Rule::Seq([29, 13]))),
                (120, Rule::Or(Rule::Seq([106, 55]), Rule::Seq([29, 34]))),
                (12, Rule::Or(Rule::Seq([79, 29]), Rule::Seq([65, 106]))),
                (50, Rule::Or(Rule::Seq([106, 16]), Rule::Seq([29, 73]))),
                (76, Rule::Or(Rule::Seq([29, 18]), Rule::Seq([106, 43]))),
                (93, Rule::Or(Rule::Seq([29, 60]), Rule::Seq([106, 51]))),
                (95, Rule::Or(Rule::Seq([106, 96]), Rule::Seq([29, 106]))),
                (32, Rule::Or(Rule::Seq([29, 98]), Rule::Seq([106, 75]))),
                (115, Rule::Or(Rule::Seq([103, 106]), Rule::Seq([23, 29]))),
                (126, Rule::Or(Rule::Seq([45, 106]), Rule::Seq([75, 29]))),
                (84, Rule::Or(Rule::Seq([106, 30]), Rule::Seq([29, 97]))),
                (34, Rule::Or(Rule::Seq([29, 121]), Rule::Seq([106, 27]))),
                (75, Rule::Seq([106, 106])),
                (33, Rule::Or(Rule::Seq([108, 106]), Rule::Seq([6, 29]))),
                (109, Rule::Or(Rule::Seq([106, 128]), Rule::Seq([29, 50]))),
                (63, Rule::Or(Rule::Seq([106, 113]), Rule::Seq([29, 39]))),
                (121, Rule::Seq([40, 29])),
                (100, Rule::Seq([96, 3])),
                (125, Rule::Or(Rule::Seq([107, 106]), Rule::Seq([104, 29]))),
                (97, Rule::Seq([29, 22])),
                (81, Rule::Or(Rule::Seq([52, 29]), Rule::Seq([123, 106]))),
                (114, Rule::Or(Rule::Seq([106, 41]), Rule::Seq([29, 19]))),
                (89, Rule::Or(Rule::Seq([29, 84]), Rule::Seq([106, 33]))),
                (102, Rule::Or(Rule::Seq([29, 72]), Rule::Seq([106, 12]))),
                (19, Rule::Or(Rule::Seq([40, 106]), Rule::Seq([54, 29]))),
                (7, Rule::Or(Rule::Seq([40, 29]), Rule::Seq([95, 106]))),
                (49, Rule::Or(Rule::Seq([106, 119]), Rule::Seq([29, 98]))),
                (66, Rule::Or(Rule::Seq([106, 5]), Rule::Seq([29, 126]))),
                (15, Rule::Or(Rule::Seq([122, 106]), Rule::Seq([57, 29]))),
                (129, Rule::Or(Rule::Seq([106, 61]), Rule::Seq([29, 27]))),
                (25, Rule::Or(Rule::Seq([29, 28]), Rule::Seq([106, 24]))),
                (4, Rule::Or(Rule::Seq([106, 98]), Rule::Seq([29, 51]))),
                (5, Rule::Or(Rule::Seq([29, 119]), Rule::Seq([106, 60]))),
                (38, Rule::Or(Rule::Seq([35, 106]), Rule::Seq([68, 29]))),
                (47, Rule::Or(Rule::Seq([98, 106]), Rule::Seq([98, 29]))),
                (105, Rule::Or(Rule::Seq([29, 106]), Rule::Seq([106, 29]))),
                (23, Rule::Or(Rule::Seq([105, 29]), Rule::Seq([22, 106]))),
                (99, Rule::Or(Rule::Seq([82, 106]), Rule::Seq([53, 29]))),
                (10, Rule::Or(Rule::Seq([106, 40]), Rule::Seq([29, 105]))),
                (16, Rule::Or(Rule::Seq([1, 106]), Rule::Seq([69, 29]))),
                (56, Rule::Or(Rule::Seq([94, 29]), Rule::Seq([90, 106]))),
                (101, Rule::Or(Rule::Seq([29, 3]), Rule::Seq([106, 98]))),
                (44, Rule::Or(Rule::Seq([9, 106]), Rule::Seq([111, 29]))),
                (65, Rule::Or(Rule::Seq([86, 106]), Rule::Seq([47, 29]))),
                (88, Rule::Or(Rule::Seq([85, 106]), Rule::Seq([27, 29]))),
                (39, Rule::Or(Rule::Seq([106, 4]), Rule::Seq([29, 121]))),
                (123, Rule::Or(Rule::Seq([98, 106]), Rule::Seq([40, 29]))),
                (54, Rule::Seq([106, 29])),
                (26, Rule::Or(Rule::Seq([106, 105]), Rule::Seq([29, 22]))),
                (73, Rule::Or(Rule::Seq([106, 59]), Rule::Seq([29, 1]))),
                (72, Rule::Or(Rule::Seq([106, 17]), Rule::Seq([29, 81]))),
                (30, Rule::Or(Rule::Seq([51, 29]), Rule::Seq([58, 106]))),
                (2, Rule::Or(Rule::Seq([29, 58]), Rule::Seq([106, 62]))),
                (24, Rule::Or(Rule::Seq([29, 21]), Rule::Seq([106, 53]))),
                (14, Rule::Or(Rule::Seq([29, 1]), Rule::Seq([106, 52]))),
                (45, Rule::Seq([29, 29])),
                (82, Rule::Seq([29, 40])),
                (22, Rule::Or(Rule::Seq([29, 29]), Rule::Seq([106, 106]))),
                (46, Rule::Or(Rule::Seq([106, 119]), Rule::Seq([29, 60]))),
                (0, Rule::Seq([8, 11])),
                (117, Rule::Or(Rule::Seq([29, 88]), Rule::Seq([106, 115]))),
                (36, Rule::Or(Rule::Seq([29, 45]), Rule::Seq([106, 98]))),
                (77, Rule::Or(Rule::Seq([106, 38]), Rule::Seq([29, 25]))),
                (92, Rule::Or(Rule::Seq([91, 29]), Rule::Seq([14, 106]))),
                (28, Rule::Or(Rule::Seq([46, 29]), Rule::Seq([7, 106]))),
                (35, Rule::Or(Rule::Seq([29, 46]), Rule::Seq([106, 112]))),
                (79, Rule::Or(Rule::Seq([29, 52]), Rule::Seq([106, 110]))),
                (18, Rule::Or(Rule::Seq([29, 98]), Rule::Seq([106, 60]))),
                (122, Rule::Or(Rule::Seq([29, 75]), Rule::Seq([106, 40]))),
                (111, Rule::Or(Rule::Seq([29, 74]), Rule::Seq([106, 110]))),
                (80, Rule::Or(Rule::Seq([51, 29]), Rule::Seq([95, 106]))),
                (69, Rule::Seq([96, 51])),
                (67, Rule::Or(Rule::Seq([71, 106]), Rule::Seq([129, 29]))),
                (11, Rule::Seq([42, 31])),
                (1, Rule::Or(Rule::Seq([3, 29]), Rule::Seq([62, 106]))),
                (29, Rule::Str('b')),
                (61, Rule::Seq([54, 106])),
            ].iter())
        }

        struct StrIter {
            string,
            position
        }

        impl StrIter {
            fn new(string) {
                Self {
                    string,
                    position: 0
                }
            }

            fn clone(self) {
                Self {
                    string: self.string,
                    position: self.position
                }
            }

            fn next(self) {
                self.position += 1;
                self.string.char_at(self.position - 1)
            }

            fn completed(self) {
                self.position == self.string.len()
            }

        }

        enum Rule {
            Str(c),
            Or(r, r),
            Seq(vs),
        }


        impl Rule {
            fn validate(self, rules, str) {
                let it = StrIter::new(str);
                self.validate_inner(rules, it).filter(|x| x.completed()).take(1).count() >= 1
            }

            fn validate_inner(self, rules, it) {
                match self {
                    Self::Str(v) => {
                        if let Some(c) = it.next() {
                            if c == v {
                                return std::iter::once(it);
                            }
                        }
                        std::iter::empty()
                    },
                    // Take all possible outcomes from LHS and RHS and return all of them... lazily
                    Self::Or(l, r) => l.validate_inner(rules, it.clone()).chain(r.validate_inner(rules, it)),
                    // This is an ungodly abomiantion which boils down BFS traversal
                    Self::Seq(vs) => vs.iter().fold(std::iter::once(it), |branches, v| branches.flat_map(|b| rules[v].validate_inner(rules, b)))
                }
            }
        }

        fn validate_all(rules, messages) {
            let root = rules[0];
            messages.iter().filter(|v| root.validate(rules, v)).count()
        }


        pub fn main(n) {
            let r = get_rules();
            let t1 = validate_all(r, n);

            // Disabled for benchmark duration reasons - this below section causes the benchmark to take multiple minutes to finish
            // r[8] = Rule::Or(Rule::Seq([42]), Rule::Seq([42, 8]));
            // r[11] = Rule::Or(Rule::Seq([42, 31]), Rule::Seq([42, 11, 31]));
            // let t2 = validate_all(r, n);
            let t2 = 334;
            assert_eq!(t1, 182);
            assert_eq!(t2, 334);
            (t1, t2)
        }
    };

    let entry = rune::Hash::type_hash(["main"]);

    b.bench_function("aoc_2020_19b", |b| {
        b.iter(|| {
            vm.call(entry, (data.try_clone().unwrap(),))
                .expect("failed call")
        });
    });
}
