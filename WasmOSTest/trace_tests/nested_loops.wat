;; nested_loops.wat — O(n³) nested loop to generate medium-duration traces
;;
;; Purpose: Produces a medium-duration trace (~10–100 ms). Tests:
;;   - Mid-range waterfall bars
;;   - Duration variance when compared with fast_add and slow_fibonacci
;;   - Instruction count tag in execute span
;;
;; Expected trace behavior:
;;   - success = true
;;   - total_duration_us between 10 000 and 500 000
;;   - High instruction count
(module
  (func $triple_loop (param $n i32) (result i32)
    (local $i i32)
    (local $j i32)
    (local $k i32)
    (local $sum i32)

    i32.const 0 local.set $sum
    i32.const 0 local.set $i

    block $exit_i
      loop $loop_i
        local.get $i
        local.get $n
        i32.ge_s
        br_if $exit_i

        i32.const 0 local.set $j
        block $exit_j
          loop $loop_j
            local.get $j
            local.get $n
            i32.ge_s
            br_if $exit_j

            i32.const 0 local.set $k
            block $exit_k
              loop $loop_k
                local.get $k
                local.get $n
                i32.ge_s
                br_if $exit_k

                ;; sum += i + j + k
                local.get $sum
                local.get $i
                local.get $j
                i32.add
                local.get $k
                i32.add
                i32.add
                local.set $sum

                local.get $k
                i32.const 1
                i32.add
                local.set $k
                br $loop_k
              end
            end

            local.get $j
            i32.const 1
            i32.add
            local.set $j
            br $loop_j
          end
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop_i
      end
    end

    local.get $sum
  )

  (func $start (export "start") (result i32)
    i32.const 50    ;; 50³ = 125 000 iterations
    call $triple_loop
  )
)
