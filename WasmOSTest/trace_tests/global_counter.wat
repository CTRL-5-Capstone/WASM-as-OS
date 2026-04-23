;; global_counter.wat — mutable global that counts loop iterations
;;
;; Purpose: Tests the "validation" span phase (globals must be validated).
;;   - Exercises mutable global read/write in a tight loop
;;   - Medium duration, success case
;;   - Environment generator will deterministically produce different
;;     sensor/vFS/envVar combos from the resulting trace_id
;;
;; Expected trace behavior:
;;   - success = true
;;   - Moderate duration
;;   - Global validation visible in trace if backend validates globals
(module
  (global $counter (mut i32) (i32.const 0))

  (func $count_to (param $n i32)
    (local $i i32)
    i32.const 0
    local.set $i

    block $exit
      loop $loop
        local.get $i
        local.get $n
        i32.ge_s
        br_if $exit

        global.get $counter
        i32.const 1
        i32.add
        global.set $counter

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
  )

  (func $start (export "start") (result i32)
    i32.const 10000
    call $count_to
    global.get $counter  ;; returns 10000
  )
)
