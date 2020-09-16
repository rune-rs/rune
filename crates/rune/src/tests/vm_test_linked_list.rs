#[test]
fn test_linked_list() {
    assert_eq! {
        rune! {
            Vec<i64> => r#"
            /// An empty placeholder in a node.
            struct Empty;

            /// A single node in the linked list.
            struct Node {
                value,
                next,
            }

            /// The linked list.
            struct List {
                first,
                last,
            }

            impl List {
                /// Construct a new linked list.
                fn new() {
                    List {
                        first: Empty,
                        last: Empty,
                    }
                }

                /// Construct an iterator over the linked list.
                fn iter(self) {
                    Iter {
                        current: self.first,
                    }
                }

                /// Push an element to the back of the linked list.
                fn push_back(self, value) {
                    let prev = self.last;

                    self.last = Node {
                        value,
                        next: Empty,
                    };

                    if prev is Empty {
                        self.first = self.last;
                    } else {
                        prev.next = self.last;
                    }
                }
            }

            struct Iter {
                current,
            }

            impl Iter {
                /// Iterate over the next element.
                fn next(self) {
                    if self.current is Empty {
                        return None;
                    }

                    let value = self.current;
                    self.current = value.next;
                    Some(value.value)
                }
            }

            fn main() {
                let ll = List::new();
                ll.push_back(1);
                ll.push_back(2);
                ll.push_back(3);

                let it = ll.iter();

                let out = [];

                while let Some(value) = Iter::next(it) {
                    out.push(value);
                }

                out
            }
            "#
        },
        vec![1, 2, 3],
    };
}
