;; divide_by_zero.wat — triggers a WASM trap on integer divide by zero
;;
;; Purpose: Produces a FAILED trace. Use this to verify:
;;   - Red "Failed" heatmap cells
;;   - Red AlertTriangle icon in the trace row
;;   - Error string displayed in span detail
;;   - "FAIL" / "Assertion Failed" badges depending on env generation
;;   - Forensic snapshot button appears
;;   - Error rate bump in Live Metrics
;;
;; Expected trace behavior:
;;   - success = false
;;   - error = something like "integer divide by zero" or "trap"
;;   - Short duration (fails fast)
(module
  (func $start (export "start") (result i32)
    i32.const 42
    i32.const 0
    i32.div_s  ;; trap: integer divide by zero
  )
)
