#[test]
fn test_basic_task_queueing() {
    // Pretend we have 3 tasks in the queue
    let mut queue: Vec<u32> = vec![1, 2, 3];

    assert_eq!(queue.len(), 3, "Queue should contain 3 tasks");
    assert_eq!(queue[0], 1, "First task ID should be 1");
}

#[test]
fn test_round_robin_rotation() {
    let mut queue: Vec<u32> = vec![1, 2, 3];

    // simulate yield: pop front, push back
    let first = queue.remove(0);
    queue.push(first);

    assert_eq!(queue, vec![2, 3, 1]);
}
