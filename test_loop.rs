extern "C" {
    fn host_log(ptr: *const u8, len: usize);
}

fn main() {
    let mut sum = 0;
    for i in 1..=5 {
        sum += i;
    }
    
    if sum == 15 {
        let msg = "Loop Test Passed: Sum of 1..5 = 15";
        unsafe {
            host_log(msg.as_ptr(), msg.len());
        }
    } else {
        let msg = "Loop Test Failed";
        unsafe {
            host_log(msg.as_ptr(), msg.len());
        }
    }
}
