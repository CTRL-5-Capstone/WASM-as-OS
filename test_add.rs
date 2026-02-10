extern "C" {
    fn host_log(ptr: *const u8, len: usize);
}

fn main() {
    let a = 10;
    let b = 20;
    let c = a + b;
    
    if c == 30 {
        let msg = "Addition Test Passed: 10 + 20 = 30";
        unsafe {
            host_log(msg.as_ptr(), msg.len());
        }
    } else {
        let msg = "Addition Test Failed";
        unsafe {
            host_log(msg.as_ptr(), msg.len());
        }
    }
}
