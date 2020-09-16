import time

class Node:
    def __init__(self, value, next):
        self.value = value
        self.next = next

class List:
    def __init__(self):
        self.first = None
        self.last = None

    def push_back(self, value):
        prev = self.last
        self.last = Node(value, None)

        if prev is None:
            self.first = self.last
        else:
            prev.next = self.last

    def __iter__(self):
        return Iter(self.first)

class Iter:
    def __init__(self, current):
        self.current = current

    def __iter__(self):
        return self

    def __next__(self):
        value = self.current

        if value is None:
            raise StopIteration()

        self.current = value.next
        return value.value

start_time = time.time()

ll = List()
ll.push_back(1)
ll.push_back(2)
ll.push_back(3)

out = list()

for value in ll:
    out.append(value)

elapsed_time = time.time() - start_time
print(out)
print("{0}ms".format(round(elapsed_time * 1_000_000) / 1000))