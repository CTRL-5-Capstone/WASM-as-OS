(module
  ;; One page of memory (64 KiB), exported so a host can inspect it if needed.
  (memory (export "memory") 1)

  ;; sum_up_to(n): returns 1 + 2 + ... + n  (for n > 0, otherwise 0)
  (func $sum_up_to (export "sum_up_to")
    (param $n i32) (result i32)
    (local $i i32) (local $acc i32)

    ;; if (n < 0) return 0;
    local.get $n
    i32.const 0
    i32.lt_s
    if
      i32.const 0
      return
    end

    ;; i = n; acc = 0;
    local.get $n
    local.set $i
    i32.const 0
    local.set $acc

    block $exit
      loop $loop
        ;; acc += i;
        local.get $acc
        local.get $i
        i32.add
        local.set $acc

        ;; i = i - 1;
        local.get $i
        i32.const 1
        i32.sub
        local.tee $i

        ;; while (i > 0) continue; else break;
        i32.const 0
        i32.gt_s
        br_if $loop
      end
    end

    local.get $acc
  )

  ;; find_max(a, b): returns max(a, b)
  (func $find_max (export "find_max")
    (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.gt_s
    if (result i32)
      local.get $a
    else
      local.get $b
    end
  )

  ;; init_array(base, len):
  ;;   for i in 0..len-1:
  ;;       *(int32*)(base + 4*i) = i;
  (func $init_array (export "init_array")
    (param $base i32) (param $len i32)
    (local $i i32)

    i32.const 0
    local.set $i

    block $exit
      loop $loop
        ;; if (i >= len) break;
        local.get $i
        local.get $len
        i32.ge_s
        br_if $exit

        ;; store i at base + 4*i
        local.get $base
        local.get $i
        i32.const 4
        i32.mul
        i32.add      ;; address
        local.get $i
        i32.store

        ;; i++
        local.get $i
        i32.const 1
        i32.add
        local.set $i

        br $loop
      end
    end
  )

  ;; dot_product(a, b, len):
  ;;   acc = 0;
  ;;   for i in 0..len-1:
  ;;       acc += a[i] * b[i];
  ;;   return acc;
  ;; where a[i] and b[i] are i32 at base + 4*i
  (func $dot_product (export "dot_product")
    (param $a i32) (param $b i32) (param $len i32)
    (result i32)
    (local $i i32) (local $acc i32)
    (local $va i32) (local $vb i32)

    i32.const 0
    local.set $i
    i32.const 0
    local.set $acc

    block $exit
      loop $loop
        ;; if (i >= len) break;
        local.get $i
        local.get $len
        i32.ge_s
        br_if $exit

        ;; va = a[i];
        local.get $a
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.load
        local.set $va

        ;; vb = b[i];
        local.get $b
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.load
        local.set $vb

        ;; acc += va * vb;
        local.get $acc
        local.get $va
        local.get $vb
        i32.mul
        i32.add
        local.set $acc

        ;; i++
        local.get $i
        i32.const 1
        i32.add
        local.set $i

        br $loop
      end
    end

    local.get $acc
  )
)
