;; unreachable_trap.wat — hits the `unreachable` instruction immediately
;;
;; Purpose: Produces a failed trace with a "trap: unreachable" error.
;;   - Different error string from divide_by_zero (tests error variety)
;;   - Very fast failure (near-zero duration)
;;   - Tests Policy Violation / Assertion Failed badge generation
;;   - Tests forensic snapshot capture on failed traces
;;
;; Expected trace behavior:
;;   - success = false
;;   - error contains "unreachable"
;;   - total_duration_us < 100
(module
  (func $start (export "start") (result i32)
    unreachable
  )
)
