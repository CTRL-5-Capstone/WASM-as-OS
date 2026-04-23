;; fibonacci.wat — recursive + iterative Fibonacci, self-verifying
;;
;; Exports:
;;   fib_iterative(n) -> i32
;;   fib_recursive(n) -> i32
;;   start()         -> i32   (returns 1 if all assertions pass, 0 otherwise)
(module
  (memory (export "memory") 1)

  ;; ── Recursive Fibonacci ────────────────────────────────────────────────────
  (func $fib_recursive (export "fib_recursive")
        (param $n i32) (result i32)
    local.get $n
    i32.const 2
    i32.lt_s
    if (result i32)
      local.get $n
    else
      local.get $n
      i32.const 1
      i32.sub
      call $fib_recursive

      local.get $n
      i32.const 2
      i32.sub
      call $fib_recursive

      i32.add
    end
  )

  ;; ── Iterative Fibonacci ────────────────────────────────────────────────────
  (func $fib_iterative (export "fib_iterative")
        (param $n i32) (result i32)
    (local $a   i32)
    (local $b   i32)
    (local $tmp i32)
    (local $i   i32)

    i32.const 0  local.set $a     ;; a = fib(0) = 0
    i32.const 1  local.set $b     ;; b = fib(1) = 1
    i32.const 0  local.set $i

    block $exit
      loop $loop
        local.get $i
        local.get $n
        i32.ge_s
        br_if $exit

        ;; tmp = a + b;  a = b;  b = tmp;
        local.get $a
        local.get $b
        i32.add       local.set $tmp
        local.get $b  local.set $a
        local.get $tmp local.set $b

        local.get $i
        i32.const 1
        i32.add       local.set $i

        br $loop
      end
    end

    local.get $a
  )

  ;; ── Store known Fibonacci table in memory ──────────────────────────────────
  ;; fib[0..12] at byte offset 0, each i32 = 4 bytes
  (func $init_fib_table
    ;; fib[0]=0, [1]=1, [2]=1, [3]=2, [4]=3, [5]=5,
    ;; [6]=8, [7]=13,[8]=21,[9]=34,[10]=55,[11]=89,[12]=144
    i32.const 0   i32.const 0   i32.store
    i32.const 4   i32.const 1   i32.store
    i32.const 8   i32.const 1   i32.store
    i32.const 12  i32.const 2   i32.store
    i32.const 16  i32.const 3   i32.store
    i32.const 20  i32.const 5   i32.store
    i32.const 24  i32.const 8   i32.store
    i32.const 28  i32.const 13  i32.store
    i32.const 32  i32.const 21  i32.store
    i32.const 36  i32.const 34  i32.store
    i32.const 40  i32.const 55  i32.store
    i32.const 44  i32.const 89  i32.store
    i32.const 48  i32.const 144 i32.store
  )

  ;; ── Cross-validate iterative vs memory table ───────────────────────────────
  ;; Returns 1 if all fib(0..12) match, 0 if any mismatch.
  (func $cross_validate (result i32)
    (local $n    i32)
    (local $got  i32)
    (local $want i32)

    i32.const 0  local.set $n

    block $fail
      loop $loop
        local.get $n
        i32.const 13
        i32.ge_s
        br_if $fail   ;; all 13 passed — break out of loop as "pass"

        ;; got = fib_iterative(n)
        local.get $n
        call $fib_iterative
        local.set $got

        ;; want = mem[n*4]
        local.get $n
        i32.const 4
        i32.mul
        i32.load
        local.set $want

        ;; if got != want → return 0
        local.get $got
        local.get $want
        i32.ne
        if
          i32.const 0
          return
        end

        local.get $n
        i32.const 1
        i32.add
        local.set $n

        br $loop
      end
    end

    i32.const 1   ;; all matched
  )

  ;; ── Entry point ───────────────────────────────────────────────────────────
  (func $start (export "start") (result i32)
    call $init_fib_table
    call $cross_validate
  )
)
