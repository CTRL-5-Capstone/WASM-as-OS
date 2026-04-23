;; slow_fibonacci.wat — recursive Fibonacci(25) to burn CPU time
;;
;; Purpose: Produces a long-running trace so we can test:
;;   - Amber "warning" heatmap cells (high duration, still success)
;;   - Wide waterfall bars
;;   - p95/p99 impact in Live Metrics
;;   - Duration sort behavior in the trace list
;;
;; Expected trace behavior:
;;   - total_duration_us in the hundreds of ms range
;;   - success = true
;;   - High instruction count in execute span tags
(module
  (func $fib (param $n i32) (result i32)
    local.get $n
    i32.const 2
    i32.lt_s
    if (result i32)
      local.get $n
    else
      local.get $n
      i32.const 1
      i32.sub
      call $fib

      local.get $n
      i32.const 2
      i32.sub
      call $fib

      i32.add
    end
  )

  (func $start (export "start") (result i32)
    i32.const 25
    call $fib  ;; fib(25) = 75025 — takes a while recursively
  )
)
