;; multi_function.wat — calls multiple internal functions in sequence
;;
;; Purpose: Produces a trace with many child spans when the tracer is wired.
;;   - Tests waterfall rendering with 4+ distinct function calls
;;   - Tests span ordering in the timeline
;;   - Each function does different work: math, memory, control flow
;;
;; Expected trace behavior:
;;   - success = true
;;   - Multiple distinct span kinds visible in waterfall
;;   - Medium instruction count
(module
  (memory (export "memory") 1)

  ;; Step 1: Compute factorial of 10
  (func $factorial (param $n i32) (result i32)
    (local $result i32)
    i32.const 1
    local.set $result

    block $exit
      loop $loop
        local.get $n
        i32.const 1
        i32.le_s
        br_if $exit

        local.get $result
        local.get $n
        i32.mul
        local.set $result

        local.get $n
        i32.const 1
        i32.sub
        local.set $n
        br $loop
      end
    end
    local.get $result
  )

  ;; Step 2: Write a value to memory
  (func $store_result (param $addr i32) (param $val i32)
    local.get $addr
    local.get $val
    i32.store
  )

  ;; Step 3: Read back and verify
  (func $load_and_check (param $addr i32) (param $expected i32) (result i32)
    local.get $addr
    i32.load
    local.get $expected
    i32.eq
  )

  ;; Step 4: Sum of first N natural numbers
  (func $sum_to (param $n i32) (result i32)
    (local $i i32)
    (local $s i32)
    i32.const 1 local.set $i
    i32.const 0 local.set $s

    block $exit
      loop $loop
        local.get $i
        local.get $n
        i32.gt_s
        br_if $exit

        local.get $s
        local.get $i
        i32.add
        local.set $s

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end

    local.get $s
  )

  (func $start (export "start") (result i32)
    ;; Step 1: 10! = 3628800
    i32.const 10
    call $factorial

    ;; Step 2: store at address 0
    i32.const 0
    i32.const 3628800
    call $store_result

    ;; Step 3: verify
    i32.const 0
    i32.const 3628800
    call $load_and_check  ;; returns 1 if ok

    ;; Step 4: sum 1..100 = 5050
    i32.const 100
    call $sum_to

    ;; Combine: check_result + sum_result
    i32.add   ;; 1 + 5050 = 5051
  )
)
