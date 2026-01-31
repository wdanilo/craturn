use std::thread;
use std::time::Duration;
use std::time::Instant;

craturn::awaken!(Hungry);

fn main() {
    println!("Craturn sanity test");

    let timeout = Duration::from_secs(15);

    println!("Vec corruption test.");
    let v: Vec<u64> = (0..10_000).collect();
    let expected_sum: u64 = (0..10_000u64).sum();
    let start = Instant::now();
    loop {
        println!(".");
        thread::sleep(Duration::from_millis(50));

        let sum: u64 = v.iter().sum();
        if sum != expected_sum {
            println!(
                "🔥 Vec corrupted after {:?}: expected {}, got {}",
                start.elapsed(),
                expected_sum,
                sum
            );
            break;
        }

        if start.elapsed() > timeout {
            println!("No visible corruption after {:?} (this run)", timeout);
            break;
        }
    }

    println!("String corruption test.");
    let content = "the quick brown fox ";
    let s_expected = content.repeat(10);
    let s = s_expected.clone();
    let start = Instant::now();
    loop {
        thread::sleep(Duration::from_millis(50));

        if s != s_expected {
            println!("🔥 String corrupted after {:?}", start.elapsed());
            println!("{s_expected:?}");
            println!("{s:?}");
            break;
        }

        if start.elapsed() > timeout {
            println!("No visible corruption after {:?} (this run)", timeout);
            break;
        }
    }

    println!("End.");
}
