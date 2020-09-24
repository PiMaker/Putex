# Putex - Pi's custom Mutex

Experimenting with how a Mutex and synchronization primitives (atomics) work.

No real use case, but short and clean Rust code for you to look at :)

### Benchmarking

Two surprises for me: The spinlock variant is *slower* than the one yielding to
the OS scheduler, but, secondly, both Putex variants are faster than a
std::sync::Mutex.

```
test tests::bench_putex_spin  ... bench:     473,454 ns/iter (+/- 28,847)
test tests::bench_putex_yield ... bench:     391,341 ns/iter (+/- 63,764)
test tests::bench_std_mutex   ... bench:     997,582 ns/iter (+/- 115,092)
```
(tested on an AMD 3960X)

# License

This repository is available in the public domain according to the CC0 license.
Do with it whatever you want, I expressively waive all ownership rights related
to this work. Also, if it makes your computer go kaboom, that's on you.
