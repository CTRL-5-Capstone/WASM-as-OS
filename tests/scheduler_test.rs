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
#[test]
fn test_round_robin_single_task() {
    let mut queue: Vec<u32> = vec![42];

    let first = queue.remove(0);
    queue.push(first);

    // With only one item, the order shouldn't change
    assert_eq!(queue, vec![42]);
}

#[test]
fn test_round_robin_empty_queue() {
    let mut queue: Vec<u32> = Vec::new();

    // Nothing to rotate, just make sure it stays empty
    assert!(queue.is_empty());
}

#[test]
fn test_round_robin_two_rotations() {
    let mut queue: Vec<u32> = vec![1, 2, 3, 4];

    // first rotation
    let first = queue.remove(0);
    queue.push(first);

    // second rotation
    let first = queue.remove(0);
    queue.push(first);

    assert_eq!(queue, vec![3, 4, 1, 2]);
}
