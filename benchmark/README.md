# Benchmark Functions

Each benchmark function corresponds to one of the following criteria. However, some functions can be written in Rust and compiled to Wasm and others requires the manual modification of the WAT.

| Function name        | Criteria                                                   | Manual modification (yes/no) |
|----------------------|------------------------------------------------------------|------------------------------|
| RuntimeFunction      | Needs to run close to but under 2 ms.                      | No                           |
| SizeFunction         | The module needs to be close to but under 256 KB.          | No                           |
| LinearMemoryFunction | Need to allocate close to but under linear memory of 10MB. | Yes                          |
| StackMemoryFunction  | Need to allocate close to but under stack memory of 512KB. | No                           |

Manual modification explanation:

- LinearMemoryFunction: modify the memory call at the end of the WAT file to request X number of pages, each weighting 64KB
