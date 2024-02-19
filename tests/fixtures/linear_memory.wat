(module
  (memory (export "memory") 200) ;; 200 pages of 64KiB each, 12.5MiB
  (func $_start)
  (export "_start" (func $_start))
)
