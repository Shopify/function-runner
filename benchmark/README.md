# Benchmark Functions

Each benchmark function corresponds to one of the following criteria. However, some functions can be written in Rust and compiled to Wasm and others requires the manual modification of the WAT.

| Function name          | Criteria                                                   | Manual modification (yes/no) |
| ---------------------- | ---------------------------------------------------------- | ---------------------------- |
| runtime_function       | Needs to run close to but under 5 ms.                      | No                           |
| size_function          | The module needs to be close to but under 256 KB.          | Yes                          |
| linear_memory_function | Need to allocate close to but under linear memory of 10MB. | Yes                          |
| stack_memory_function  | Need to allocate close to but under stack memory of 512KB. | No                           |

Manual modification explanation:

- linear_memory_function: modified the memory call at line 73646 of the `linear_memory_function.wasm` file to request 159 pages, each weighting 64KB. 160 pages is 10MB, but since we want to be under the limit we remove 1 from 160 which gives 159.

- size_function: added a constant at the end of the file consisting of a 160KB long string.

## The runtime Function

Functions will have different runtimes depending on whether or not they're run locally or in production due to differences in available computing power. This runtime_function benchmark is configured to take just below the 5ms timeout in production, though it may execute faster locally. You can compare a function against this benchmark to determine if it is likely to timeout in production. 
