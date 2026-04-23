;; bubble_sort_large.wat — bubble sort on a 200-element array
;;
;; Purpose: CPU-intensive O(n²) sort to produce a long trace with:
;;   - High instruction count
;;   - Significant memory usage (800+ bytes for array)
;;   - Good candidate for p95/p99 outlier detection
;;   - Tests the amber "Warning" heatmap cell (success but slow)
;;
;; Expected trace behavior:
;;   - success = true
;;   - total_duration_us in the 50 000 – 500 000 range
;;   - memory_used_bytes > 0 (uses 1 page)
(module
  (memory (export "memory") 1)

  ;; Fill mem[0..len*4] with descending values
  (func $init (param $len i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $exit
      loop $loop
        local.get $i local.get $len i32.ge_s br_if $exit
        ;; mem[i*4] = len - 1 - i
        local.get $i i32.const 4 i32.mul
        local.get $len i32.const 1 i32.sub local.get $i i32.sub
        i32.store
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
  )

  ;; Bubble sort ascending
  (func $sort (param $len i32)
    (local $i i32) (local $j i32)
    (local $addr_a i32) (local $addr_b i32)
    (local $va i32) (local $vb i32)

    i32.const 0 local.set $i
    block $exit_i
      loop $loop_i
        local.get $i local.get $len i32.const 1 i32.sub i32.ge_s br_if $exit_i

        i32.const 0 local.set $j
        block $exit_j
          loop $loop_j
            local.get $j
            local.get $len i32.const 1 i32.sub local.get $i i32.sub
            i32.ge_s br_if $exit_j

            local.get $j i32.const 4 i32.mul local.set $addr_a
            local.get $j i32.const 1 i32.add i32.const 4 i32.mul local.set $addr_b

            local.get $addr_a i32.load local.set $va
            local.get $addr_b i32.load local.set $vb

            local.get $va local.get $vb i32.gt_s
            if
              local.get $addr_a local.get $vb i32.store
              local.get $addr_b local.get $va i32.store
            end

            local.get $j i32.const 1 i32.add local.set $j
            br $loop_j
          end
        end

        local.get $i i32.const 1 i32.add local.set $i
        br $loop_i
      end
    end
  )

  ;; Check ascending order
  (func $is_sorted (param $len i32) (result i32)
    (local $i i32)
    i32.const 1 local.set $i
    block $exit
      loop $loop
        local.get $i local.get $len i32.ge_s br_if $exit
        local.get $i i32.const 1 i32.sub i32.const 4 i32.mul i32.load
        local.get $i i32.const 4 i32.mul i32.load
        i32.gt_s
        if
          i32.const 0 return
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    i32.const 1
  )

  (func $start (export "start") (result i32)
    i32.const 200
    call $init
    i32.const 200
    call $sort
    i32.const 200
    call $is_sorted  ;; returns 1 if correctly sorted
  )
)
