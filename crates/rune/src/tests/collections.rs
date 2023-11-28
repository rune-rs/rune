prelude!();

#[test]
fn test_hash_map_tile() {
    rune! {
        pub fn main() {
            use std::collections::HashMap;

            enum Tile {
                Wall,
            }

            let m = HashMap::new();

            m.insert((0, 1), Tile::Wall);
            m[(0, 3)] = 5;

            assert_eq!(m.get((0, 1)), Some(Tile::Wall));
            assert_eq!(m.get((0, 2)), None);
            assert_eq!(m[(0, 3)], 5);
        }
    };
}

#[test]
fn test_hash_set_tuple() {
    rune! {
        pub fn main() {
            use std::collections::HashSet;

            enum Tile {
                Wall,
            }

            let m = HashSet::new();

            m.insert((0, 1));

            assert!(m.contains((0, 1)));
            assert!(!m.contains((0, 2)));
        }
    };
}
