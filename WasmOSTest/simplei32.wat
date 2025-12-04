(module
  (func $add (param $x i32) (param $y i32) (result i32)
    local.get $x
    local.get $y
    i32.add
  )

  (func $start (export "start") (result i32)
    i32.const 5
    i32.const 7
    call $add  ;; 5 + 7 = 12
    return
  )
)