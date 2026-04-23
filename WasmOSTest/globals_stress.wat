;; globals_stress.wat — exercises mutable globals, counters, accumulators
;;
;; Tests: global reads/writes, interaction with locals, loops updating globals.
(module
  ;; ── Globals ───────────────────────────────────────────────────────────────
  (global $counter   (mut i32) (i32.const 0))
  (global $sum       (mut i32) (i32.const 0))
  (global $product   (mut i32) (i32.const 1))
  (global $max_seen  (mut i32) (i32.const -2147483648))  ;; i32::MIN
  (global $min_seen  (mut i32) (i32.const 2147483647))   ;; i32::MAX
  (global $flag      (mut i32) (i32.const 0))

  ;; Count 1..n — updates $counter and $sum
  (func $count_to (param $n i32)
    (local $i i32)
    i32.const 1  local.set $i

    block $exit
      loop $loop
        local.get $i  local.get $n  i32.gt_s  br_if $exit

        ;; counter++
        global.get $counter
        i32.const 1
        i32.add
        global.set $counter

        ;; sum += i
        global.get $sum
        local.get $i
        i32.add
        global.set $sum

        local.get $i  i32.const 1  i32.add  local.set $i
        br $loop
      end
    end
  )

  ;; Multiply 1*2*...*n (factorial) stored in $product
  (func $factorial (param $n i32)
    (local $i i32)
    i32.const 1  global.set $product
    i32.const 1  local.set $i

    block $exit
      loop $loop
        local.get $i  local.get $n  i32.gt_s  br_if $exit

        global.get $product
        local.get $i
        i32.mul
        global.set $product

        local.get $i  i32.const 1  i32.add  local.set $i
        br $loop
      end
    end
  )

  ;; Scan values 5,3,9,1,7,4,8,2,6 — track max and min
  (func $scan_minmax
    (local $v i32)

    ;; Use a helper approach: unroll 9 values into a loop via memory.
    ;; Values stored inline as a sequence of calls to a helper.
    i32.const 5   call $observe
    i32.const 3   call $observe
    i32.const 9   call $observe
    i32.const 1   call $observe
    i32.const 7   call $observe
    i32.const 4   call $observe
    i32.const 8   call $observe
    i32.const 2   call $observe
    i32.const 6   call $observe
  )

  (func $observe (param $v i32)
    ;; update max
    local.get $v  global.get $max_seen  i32.gt_s
    if
      local.get $v  global.set $max_seen
    end
    ;; update min
    local.get $v  global.get $min_seen  i32.lt_s
    if
      local.get $v  global.set $min_seen
    end
  )

  ;; Fibonacci stored entirely in globals
  (global $fib_a (mut i32) (i32.const 0))
  (global $fib_b (mut i32) (i32.const 1))
  (global $fib_tmp (mut i32) (i32.const 0))

  (func $fib_global (param $n i32) (result i32)
    (local $i i32)
    i32.const 0  global.set $fib_a
    i32.const 1  global.set $fib_b
    i32.const 0  local.set $i

    block $exit
      loop $loop
        local.get $i  local.get $n  i32.ge_s  br_if $exit

        global.get $fib_a
        global.get $fib_b
        i32.add       global.set $fib_tmp
        global.get $fib_b  global.set $fib_a
        global.get $fib_tmp global.set $fib_b

        local.get $i  i32.const 1  i32.add  local.set $i
        br $loop
      end
    end
    global.get $fib_a
  )

  ;; Entry: run all checks, return 1 if everything is correct
  (func $start (export "start") (result i32)
    ;; 1. count 1..10 → counter=10, sum=55
    i32.const 10  call $count_to
    global.get $counter  i32.const 10  i32.ne  if  i32.const 0  return  end
    global.get $sum      i32.const 55  i32.ne  if  i32.const 0  return  end

    ;; 2. factorial(7) = 5040
    i32.const 7  call $factorial
    global.get $product  i32.const 5040  i32.ne  if  i32.const 0  return  end

    ;; 3. scan 5,3,9,1,7,4,8,2,6 → max=9, min=1
    call $scan_minmax
    global.get $max_seen  i32.const 9  i32.ne  if  i32.const 0  return  end
    global.get $min_seen  i32.const 1  i32.ne  if  i32.const 0  return  end

    ;; 4. fib_global(10) = 55
    i32.const 10  call $fib_global
    i32.const 55  i32.ne  if  i32.const 0  return  end

    i32.const 1  ;; all passed
  )
)
