;; fast_add.wat — lightweight add operation, finishes in microseconds
;;
;; Purpose: Produces a short, successful trace. Use this to verify that
;;          the traces page correctly renders a green "OK" row, a tiny
;;          waterfall bar, and a valid TTFS badge.
;;
;; Expected trace behavior:
;;   - 1 root span + load + validate + execute
;;   - total_duration_us < 1 000 µs
;;   - success = true
;;   - No error string
(module
  (func $add (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add
  )

  (func $start (export "start") (result i32)
    i32.const 42
    i32.const 58
    call $add  ;; 42 + 58 = 100
  )
)
