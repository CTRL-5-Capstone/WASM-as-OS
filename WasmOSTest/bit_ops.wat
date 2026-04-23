;; bit_ops.wat — exercises bitwise / shift / rotation / count instructions
;;
;; Tests: shl, shr_s, shr_u, rotl, rotr, clz, ctz, popcnt, and/or/xor
(module

  ;; ── Utilities ─────────────────────────────────────────────────────────────

  ;; Count set bits via popcnt
  (func $popcount (param $x i32) (result i32)
    local.get $x
    i32.popcnt
  )

  ;; Number of leading zeros
  (func $leading_zeros (param $x i32) (result i32)
    local.get $x
    i32.clz
  )

  ;; Number of trailing zeros
  (func $trailing_zeros (param $x i32) (result i32)
    local.get $x
    i32.ctz
  )

  ;; Rotate left by 1 using rotl
  (func $rotl1 (param $x i32) (result i32)
    local.get $x
    i32.const 1
    i32.rotl
  )

  ;; Rotate right by 1
  (func $rotr1 (param $x i32) (result i32)
    local.get $x
    i32.const 1
    i32.rotr
  )

  ;; Isolate lowest set bit (x & -x)
  (func $lowest_bit (param $x i32) (result i32)
    local.get $x
    local.get $x
    i32.const 0
    i32.sub      ;; -x (wrapping)
    i32.and
  )

  ;; Clear lowest set bit (x & (x-1))
  (func $clear_lowest (param $x i32) (result i32)
    local.get $x
    local.get $x
    i32.const 1
    i32.sub
    i32.and
  )

  ;; XOR swap: returns a XOR b (used to verify swap without temp)
  (func $xor_check (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.xor
  )

  ;; Parity (1 if odd number of set bits, 0 if even)
  (func $parity (param $x i32) (result i32)
    local.get $x
    i32.popcnt
    i32.const 1
    i32.and
  )

  ;; ── Self-validating start ─────────────────────────────────────────────────
  (func $start (export "start") (result i32)
    ;; 1. popcnt(0xFF) = 8
    i32.const 0xFF    call $popcount  i32.const 8  i32.ne  if  i32.const 0  return  end

    ;; 2. popcnt(0) = 0
    i32.const 0       call $popcount  i32.const 0  i32.ne  if  i32.const 0  return  end

    ;; 3. popcnt(0xFFFFFFFF) = 32
    i32.const -1      call $popcount  i32.const 32 i32.ne  if  i32.const 0  return  end

    ;; 4. clz(1) = 31
    i32.const 1       call $leading_zeros  i32.const 31  i32.ne  if  i32.const 0  return  end

    ;; 5. clz(0x80000000) = 0
    i32.const -2147483648  call $leading_zeros  i32.const 0  i32.ne  if  i32.const 0  return  end

    ;; 6. ctz(8) = 3  (8 = 0b1000)
    i32.const 8       call $trailing_zeros  i32.const 3  i32.ne  if  i32.const 0  return  end

    ;; 7. rotl(1, 1) = 2
    i32.const 1       call $rotl1  i32.const 2  i32.ne  if  i32.const 0  return  end

    ;; 8. rotr(2, 1) = 1
    i32.const 2       call $rotr1  i32.const 1  i32.ne  if  i32.const 0  return  end

    ;; 9. rotr(1, 1) = 0x80000000 (wraps around)
    i32.const 1       call $rotr1
    i32.const -2147483648  ;; 0x80000000 as i32
    i32.ne  if  i32.const 0  return  end

    ;; 10. lowest_bit(12) = 4  (12 = 0b1100)
    i32.const 12      call $lowest_bit  i32.const 4  i32.ne  if  i32.const 0  return  end

    ;; 11. clear_lowest(12) = 8  (0b1100 → 0b1000)
    i32.const 12      call $clear_lowest  i32.const 8  i32.ne  if  i32.const 0  return  end

    ;; 12. parity(0b1011) = 1 (3 set bits)
    i32.const 11      call $parity  i32.const 1  i32.ne  if  i32.const 0  return  end

    ;; 13. parity(0b1100) = 0 (2 set bits)
    i32.const 12      call $parity  i32.const 0  i32.ne  if  i32.const 0  return  end

    ;; 14. shl: 1 << 4 = 16
    i32.const 1  i32.const 4  i32.shl  i32.const 16  i32.ne  if  i32.const 0  return  end

    ;; 15. shr_s: -16 >> 2 = -4  (arithmetic shift)
    i32.const -16  i32.const 2  i32.shr_s  i32.const -4  i32.ne  if  i32.const 0  return  end

    ;; 16. shr_u: -1 >> 1 = 0x7FFFFFFF (logical shift, no sign extension)
    i32.const -1  i32.const 1  i32.shr_u  i32.const 2147483647  i32.ne  if  i32.const 0  return  end

    ;; 17. xor: 0b1010 XOR 0b1100 = 0b0110 = 6
    i32.const 10  i32.const 12  call $xor_check  i32.const 6  i32.ne  if  i32.const 0  return  end

    ;; 18. and: 0xFF00 & 0x0FF0 = 0x0F00 = 3840
    i32.const 0xFF00  i32.const 0x0FF0  i32.and  i32.const 0x0F00  i32.ne  if  i32.const 0  return  end

    ;; 19. or: 0xF0 | 0x0F = 0xFF = 255
    i32.const 0xF0  i32.const 0x0F  i32.or  i32.const 0xFF  i32.ne  if  i32.const 0  return  end

    i32.const 1  ;; all 19 checks passed
  )
)
