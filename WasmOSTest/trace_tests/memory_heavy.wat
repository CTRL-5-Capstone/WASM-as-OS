;; memory_heavy.wat — allocates memory and writes a large block
;;
;; Purpose: Produces a trace with high memory_used_bytes. Tests:
;;   - Memory-related tags show up in span details
;;   - Environment heatmap shows data for memory-intensive modules
;;   - vFS-like I/O patterns are simulated by the frontend env generator
;;
;; Expected trace behavior:
;;   - success = true
;;   - memory_used_bytes > 64 KB (1 page = 64 KB, we allocate 4)
;;   - Multiple store instructions drive instruction count up
(module
  (memory (export "memory") 4)  ;; 4 pages = 256 KB

  ;; Fill first 4096 bytes with a pattern (i = value)
  (func $fill_memory (param $count i32)
    (local $i i32)
    i32.const 0
    local.set $i

    block $exit
      loop $loop
        local.get $i
        local.get $count
        i32.ge_s
        br_if $exit

        ;; mem[i * 4] = i
        local.get $i
        i32.const 4
        i32.mul
        local.get $i
        i32.store

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
  )

  ;; Verify first N words were written correctly
  (func $verify (param $count i32) (result i32)
    (local $i i32)
    i32.const 0
    local.set $i

    block $exit
      loop $loop
        local.get $i
        local.get $count
        i32.ge_s
        br_if $exit

        ;; if mem[i * 4] != i, return 0
        local.get $i
        i32.const 4
        i32.mul
        i32.load
        local.get $i
        i32.ne
        if
          i32.const 0
          return
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end

    i32.const 1  ;; all checks passed
  )

  (func $start (export "start") (result i32)
    i32.const 1024
    call $fill_memory

    i32.const 1024
    call $verify   ;; returns 1 if memory writes are correct
  )
)
