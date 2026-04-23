;; stack_overflow.wat — infinite recursion to blow the call stack
;;
;; Purpose: Produces a failed trace caused by stack exhaustion.
;;   - Tests a third distinct error type (stack overflow / call depth)
;;   - Tests that the frontend handles very short error traces gracefully
;;   - Should trigger different environment scenarios than divide_by_zero
;;
;; Expected trace behavior:
;;   - success = false
;;   - error contains "stack" or "call depth" or "recursion"
;;   - Moderate duration (burns some time before crashing)
(module
  (func $recurse (result i32)
    call $recurse
    i32.const 1
    i32.add
  )

  (func $start (export "start") (result i32)
    call $recurse
  )
)
