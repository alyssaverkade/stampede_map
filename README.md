# Stampede Map

A (eventually) concurrent hashmap.

PLEASE DON'T USE THIS FOR THE LOVE OF CTHULHU UNTIL I HAVE ELIDED THIS NOTICE

This is a hashmap that bases its design heavily off of Abseil's
[Swiss Table](https://github.com/abseil/abseil-cpp/blob/master/absl/container/flat_hash_map.h) Implementation,
but eventually I plan on adding RCU-like semantics to it.


Non Goals:
* supporting platforms other than x86_64 until I have a design I like
* API conformance with `std::collections`

