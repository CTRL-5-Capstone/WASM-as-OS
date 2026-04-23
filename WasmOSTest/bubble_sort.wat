;; bubble_sort.wat — in-place bubble sort on i32 array in linear memory
;;
;; Layout: array lives at byte offset 0, up to 64 elements (4 bytes each).
;;
;; Exports:
;;   init_array(len)            — fill mem[0..len] with descending values
;;   bubble_sort(base, len)     — sort in ascending order
;;   is_sorted(base, len) -> i32 — 1 if sorted ascending, 0 otherwise
;;   start()            -> i32  — returns 1 if the full test passes
(module
  (memory (export "memory") 1)

  ;; Fill mem[base..base+len*4] with (len-1), (len-2), ..., 1, 0  (descending)
  (func $init_array (export "init_array")
        (param $base i32) (param $len i32)
    (local $i i32)
    i32.const 0 local.set $i

    block $exit
      loop $loop
        local.get $i  local.get $len  i32.ge_s  br_if $exit

        ;; mem[base + i*4] = len - 1 - i
        local.get $base
        local.get $i  i32.const 4  i32.mul
        i32.add

        local.get $len  i32.const 1  i32.sub
        local.get $i    i32.sub

        i32.store

        local.get $i  i32.const 1  i32.add  local.set $i
        br $loop
      end
    end
  )

  ;; Classic O(n²) bubble sort, ascending
  (func $bubble_sort (export "bubble_sort")
        (param $base i32) (param $len i32)
    (local $i   i32)
    (local $j   i32)
    (local $a   i32)   ;; address of element j
    (local $b   i32)   ;; address of element j+1
    (local $va  i32)
    (local $vb  i32)

    i32.const 0  local.set $i

    block $outer_exit
      loop $outer
        local.get $i  local.get $len  i32.ge_s  br_if $outer_exit
        i32.const 0   local.set $j

        block $inner_exit
          loop $inner
            ;; j < len - 1 - i
            local.get $j
            local.get $len  i32.const 1  i32.sub
            local.get $i    i32.sub
            i32.ge_s
            br_if $inner_exit

            ;; a = base + j*4
            local.get $base
            local.get $j  i32.const 4  i32.mul
            i32.add  local.set $a

            ;; b = a + 4
            local.get $a  i32.const 4  i32.add  local.set $b

            ;; va = mem[a];  vb = mem[b]
            local.get $a  i32.load  local.set $va
            local.get $b  i32.load  local.set $vb

            ;; if va > vb: swap
            local.get $va  local.get $vb  i32.gt_s
            if
              local.get $a  local.get $vb  i32.store
              local.get $b  local.get $va  i32.store
            end

            local.get $j  i32.const 1  i32.add  local.set $j
            br $inner
          end
        end

        local.get $i  i32.const 1  i32.add  local.set $i
        br $outer
      end
    end
  )

  ;; Returns 1 if mem[base..base+len*4] is non-decreasing, else 0
  (func $is_sorted (export "is_sorted")
        (param $base i32) (param $len i32) (result i32)
    (local $i  i32)
    (local $va i32)
    (local $vb i32)

    i32.const 0  local.set $i

    block $exit
      loop $loop
        local.get $i
        local.get $len  i32.const 1  i32.sub
        i32.ge_s
        br_if $exit

        ;; va = mem[base + i*4]
        local.get $base
        local.get $i  i32.const 4  i32.mul
        i32.add
        i32.load  local.set $va

        ;; vb = mem[base + (i+1)*4]
        local.get $base
        local.get $i  i32.const 1  i32.add  i32.const 4  i32.mul
        i32.add
        i32.load  local.set $vb

        local.get $va  local.get $vb  i32.gt_s
        if
          i32.const 0
          return
        end

        local.get $i  i32.const 1  i32.add  local.set $i
        br $loop
      end
    end

    i32.const 1
  )

  ;; Verify sum of sorted array equals 0+1+…+(n-1) = n*(n-1)/2
  (func $array_sum (export "array_sum")
        (param $base i32) (param $len i32) (result i32)
    (local $i   i32)
    (local $acc i32)
    i32.const 0  local.set $i
    i32.const 0  local.set $acc

    block $exit
      loop $loop
        local.get $i  local.get $len  i32.ge_s  br_if $exit

        local.get $acc
        local.get $base
        local.get $i  i32.const 4  i32.mul
        i32.add
        i32.load
        i32.add
        local.set $acc

        local.get $i  i32.const 1  i32.add  local.set $i
        br $loop
      end
    end

    local.get $acc
  )

  ;; Entry: sort 16 descending elements, check order + sum
  (func $start (export "start") (result i32)
    (local $n       i32)
    (local $sorted  i32)
    (local $got_sum i32)
    (local $exp_sum i32)

    i32.const 16  local.set $n

    ;; fill with 15,14,...,0
    i32.const 0  local.get $n  call $init_array

    ;; sort
    i32.const 0  local.get $n  call $bubble_sort

    ;; check sorted
    i32.const 0  local.get $n  call $is_sorted  local.set $sorted

    ;; sum should be n*(n-1)/2 = 16*15/2 = 120
    i32.const 0  local.get $n  call $array_sum  local.set $got_sum
    local.get $n
    local.get $n  i32.const 1  i32.sub
    i32.mul
    i32.const 2  i32.div_s
    local.set $exp_sum

    ;; return 1 only if sorted AND sum matches
    local.get $sorted
    local.get $got_sum  local.get $exp_sum  i32.eq
    i32.and
  )
)
