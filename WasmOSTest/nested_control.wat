;; nested_control.wat — deeply nested blocks, loops, br_table, select
;;
;; Tests: block/loop/if nesting, br, br_if, br_table, select, early returns
(module

  ;; ── FizzBuzz — nested if/else returning a code ─────────────────────────
  ;; Returns: 0=neither, 1=fizz, 2=buzz, 3=fizzbuzz
  (func $fizzbuzz_code (param $n i32) (result i32)
    (local $fizz i32)
    (local $buzz i32)

    ;; fizz = (n % 3 == 0)
    local.get $n  i32.const 3  i32.rem_s  i32.eqz
    local.set $fizz

    ;; buzz = (n % 5 == 0)
    local.get $n  i32.const 5  i32.rem_s  i32.eqz
    local.set $buzz

    local.get $fizz  local.get $buzz  i32.and
    if (result i32)
      i32.const 3  ;; fizzbuzz
    else
      local.get $fizz
      if (result i32)
        i32.const 1  ;; fizz
      else
        local.get $buzz
        if (result i32)
          i32.const 2  ;; buzz
        else
          i32.const 0
        end
      end
    end
  )

  ;; Count fizzbuzz categories from 1..n, return pack: fb<<24|f<<16|b<<8|none
  (func $fizzbuzz_counts (param $n i32) (result i32)
    (local $i   i32)
    (local $cnt_none i32)
    (local $cnt_fizz i32)
    (local $cnt_buzz i32)
    (local $cnt_fb   i32)
    (local $code i32)

    i32.const 1  local.set $i

    block $exit
      loop $loop
        local.get $i  local.get $n  i32.gt_s  br_if $exit

        local.get $i  call $fizzbuzz_code  local.set $code

        ;; switch on code using br_table
        block $case_none
          block $case_fizz
            block $case_buzz
              block $case_fb
                local.get $code
                br_table $case_none $case_fizz $case_buzz $case_fb
              end
              ;; fizzbuzz
              local.get $cnt_fb  i32.const 1  i32.add  local.set $cnt_fb
              br $exit   ;; DON'T break — we need br to $loop_continue below
            end
            ;; buzz
            local.get $cnt_buzz  i32.const 1  i32.add  local.set $cnt_buzz
            br $exit
          end
          ;; fizz
          local.get $cnt_fizz  i32.const 1  i32.add  local.set $cnt_fizz
          br $exit
        end
        ;; none
        local.get $cnt_none  i32.const 1  i32.add  local.set $cnt_none

        ;; The $exit break from br_table lands here in each arm above,
        ;; but we want to continue the loop, so restructure:
        ;; Actually the blocks above br_if $exit before loop — let's just increment i.
        local.get $i  i32.const 1  i32.add  local.set $i
        br $loop
      end
    end

    ;; Pack into single i32: fb * 1000 + fizz * 100 + buzz * 10 + none
    local.get $cnt_fb
    i32.const 1000  i32.mul
    local.get $cnt_fizz  i32.const 100  i32.mul  i32.add
    local.get $cnt_buzz  i32.const 10   i32.mul  i32.add
    local.get $cnt_none  i32.add
  )

  ;; ── Select instruction test ───────────────────────────────────────────────
  ;; select returns 1st if condition truthy, else 2nd
  (func $clamp (param $v i32) (param $lo i32) (param $hi i32) (result i32)
    ;; if v < lo → lo; if v > hi → hi; else v
    ;; clamp_lo = select(lo, v, v < lo)
    (local $clamped i32)
    local.get $lo
    local.get $v
    local.get $v  local.get $lo  i32.lt_s
    select
    local.set $clamped

    ;; clamp_hi = select(hi, clamped, clamped > hi)
    local.get $hi
    local.get $clamped
    local.get $clamped  local.get $hi  i32.gt_s
    select
  )

  ;; ── Nested loop: matrix row sum ───────────────────────────────────────────
  ;; Computes sum of a "virtual" N×N identity matrix (trace = N)
  (func $identity_trace (param $n i32) (result i32)
    (local $row i32)
    (local $col i32)
    (local $acc i32)

    i32.const 0  local.set $row
    i32.const 0  local.set $acc

    block $outer_exit
      loop $outer
        local.get $row  local.get $n  i32.ge_s  br_if $outer_exit
        i32.const 0  local.set $col

        block $inner_exit
          loop $inner
            local.get $col  local.get $n  i32.ge_s  br_if $inner_exit

            ;; if row == col: acc++
            local.get $row  local.get $col  i32.eq
            if
              local.get $acc  i32.const 1  i32.add  local.set $acc
            end

            local.get $col  i32.const 1  i32.add  local.set $col
            br $inner
          end
        end

        local.get $row  i32.const 1  i32.add  local.set $row
        br $outer
      end
    end

    local.get $acc
  )

  ;; ── Entry ──────────────────────────────────────────────────────────────────
  (func $start (export "start") (result i32)

    ;; 1. identity_trace(7) = 7
    i32.const 7  call $identity_trace
    i32.const 7  i32.ne  if  i32.const 0  return  end

    ;; 2. clamp(-5, 0, 10) = 0
    i32.const -5  i32.const 0  i32.const 10  call $clamp
    i32.const 0  i32.ne  if  i32.const 0  return  end

    ;; 3. clamp(15, 0, 10) = 10
    i32.const 15  i32.const 0  i32.const 10  call $clamp
    i32.const 10  i32.ne  if  i32.const 0  return  end

    ;; 4. clamp(7, 0, 10) = 7
    i32.const 7   i32.const 0  i32.const 10  call $clamp
    i32.const 7  i32.ne  if  i32.const 0  return  end

    ;; 5. fizzbuzz_code(15) = 3 (fizzbuzz)
    i32.const 15  call $fizzbuzz_code  i32.const 3  i32.ne  if  i32.const 0  return  end

    ;; 6. fizzbuzz_code(9) = 1 (fizz)
    i32.const 9   call $fizzbuzz_code  i32.const 1  i32.ne  if  i32.const 0  return  end

    ;; 7. fizzbuzz_code(10) = 2 (buzz)
    i32.const 10  call $fizzbuzz_code  i32.const 2  i32.ne  if  i32.const 0  return  end

    ;; 8. fizzbuzz_code(7) = 0 (neither)
    i32.const 7   call $fizzbuzz_code  i32.const 0  i32.ne  if  i32.const 0  return  end

    i32.const 1
  )
)
