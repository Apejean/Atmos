fn main() {
    let (tx, rx) = crossbeam_channel::bounded::<String>(1024);
    
    // Simulate old audio engine
    let rx1 = rx.clone();
    let t1 = std::thread::spawn(move || {
        // consumes messages quickly
        let mut consumed = 0;
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < 500 {
            if let Ok(msg) = rx1.try_recv() {
                println!("Old engine consumed: {}", msg);
                consumed += 1;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        println!("Old engine exiting, consumed {}", consumed);
    });
    
    std::thread::sleep(std::time::Duration::from_millis(100)); // wait for old engine to be running
    
    // Simulate new audio engine
    let rx2 = rx.clone();
    let t2 = std::thread::spawn(move || {
        let mut consumed = 0;
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < 1000 {
            if let Ok(msg) = rx2.try_recv() {
                println!("New engine consumed: {}", msg);
                consumed += 1;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        println!("New engine exiting, consumed {}", consumed);
    });
    
    // Simulate commands sent during overlap
    for i in 0..5 {
        tx.try_send(format!("PlayTrack {}", i)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    
    t1.join().unwrap();
    t2.join().unwrap();
}
