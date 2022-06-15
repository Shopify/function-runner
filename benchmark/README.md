# Benchmark Functions

Each benchmark function corresponds to one of the following criteria. However, some functions can be written in Rust and compiled to Wasm and others requires the manual modification of the WAT.

| Function name          | Criteria                                                   | Manual modification (yes/no) |
|------------------------|------------------------------------------------------------|------------------------------|
| runtime_function       | Needs to run close to but under 2 ms.                      | No                           |
| size_function          | The module needs to be close to but under 256 KB.          | Yes                           |
| linear_memory_function | Need to allocate close to but under linear memory of 10MB. | Yes                          |
| stack_memory_function  | Need to allocate close to but under stack memory of 512KB. | No                           |

Manual modification explanation:

- linear_memory_function: modified the memory call at line 46445 of the `linear_memory_function.wasm` file to request 159 pages, each weighting 64KB. 160 pages is 10MB, but since we want to be under the limit we remove 1 from 160 which gives 159.

- size_function: added a constant at the end of the file consisting of a 160KB long string.
