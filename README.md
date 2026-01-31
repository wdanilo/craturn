<img width="698" alt="Image" src="https://github.com/user-attachments/assets/22276cf0-9eb2-4df9-91b8-106c176d0c7e" />


<br/>
<br/>

# 🪐 craturn

**A Rust interpretation of the “Saturn Devouring His Son” painting.**

`craturn` is a joke global allocator that slowly, subtly, and nondeterministically **eats
allocated memory**, resulting in corrupted program state over time, while remaining fully
valid Rust code.

It is inspired by Francisco Goya’s *Saturn Devouring His Son*.  
Except Saturn here is **your program**, and the son is **its own heap**.

The allocator behaves normally at first.  
Then it starts to *eat*.
<br/>
Sometimes nothing happens.<br/>
Sometimes bits disappear.<br/>
Sometimes values decay.<br/>
Sometimes the program hangs.<br/>
Sometimes everything is fine ... until it isn’t.<br/>

<br/>

---

# ⚠️ Disclaimer

This crate is **intentionally unsafe**, **intentionally incorrect**, and **intentionally evil**.

- Do **not** use in production.
- Do **not** use in benchmarks you care about.
- Do **not** file bugs saying “it broke my program”.

This crate exists for:
- jokes,
- demos,
- chaos testing,
- explaining why memory safety matters,
- terrifying coworkers.

You have been warned.

<br/>

---

# 🧭 What `craturn` Does

Once awakened, `craturn` installs a custom **global allocator** that:

- Allocates memory exactly like the system allocator.
- Tracks long-lived heap allocations.
- Spawns a single background “eater” thread.
- Occasionally eats bits inside live heap objects.
- Replaces eaten bits with zeros.
- Scales its appetite based on a configurable **hunger level**.

Crucially:
- There are **no panics**.
- There are **no explicit crashes**.
- Everything compiles cleanly.
- The damage appears *later*, *elsewhere*, and *without context*.

In other words:  
**your program is being consumed from the inside.**

<br/>

<br/>

# 🍖 Hunger Levels

Hunger controls **how often** and **how much** memory is eaten.

```rust
pub enum Hunger {
    Full,        // Eats nothing
    Hungry,      // Rare, tiny bites
    Starving,    // More frequent nibbling
    Devouring,   // Large chunks disappear
    Insatiable,  // Loud, fast, obvious consumption
}
```

Higher hunger:
- Eats memory more frequently.
- Removes more bits per bite.
- Converges faster to visible failure.

Lower hunger:
- May take seconds, minutes, or never.
- Is ideal for subtle, deniable breakage.

<br/>

<br/>

# 🛠️ Usage

Add `craturn` as a dependency, then **awaken it**.

```rust
craturn::awaken!();          // defaults to Hungry
// or
craturn::awaken!(Starving);  // explicit hunger
```

That’s it.
<br/>
No function calls.<br/>
No runtime handles.<br/>
No opt-out.<br/>
<br/>
Once awakened, Saturn starts eating.

<br/>

<br/>

# 🧪 Example

```rust
use std::thread;
use std::time::{Duration, Instant};

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
                "🔥 Vec eaten after {:?}: expected {}, got {}",
                start.elapsed(),
                expected_sum,
                sum
            );
            break;
        }

        if start.elapsed() > timeout {
            println!("No visible eating after {:?} (this run)", timeout);
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
            println!("🔥 String eaten after {:?}", start.elapsed());
            println!("{s_expected:?}");
            println!("{s:?}");
            break;
        }

        if start.elapsed() > timeout {
            println!("No visible eating after {:?} (this run)", timeout);
            break;
        }
    }

    println!("End.");
}
```

### Possible outcomes

- Values slowly decay to zero.
- Collections lose elements.
- Program hangs due to eaten state.
- Everything appears fine, for now.

All outcomes are correct.

<br/>

<br/>

# 🧙 Macro Details

The allocator is installed via a macro to keep activation **non-obvious**:

```rust
#[macro_export]
macro_rules! awaken {
    () => {
        $crate::awaken!(Hungry);
    };
    ($hunger:ident) => {
        #[global_allocator]
        static A: craturn::Allocator = craturn::Allocator {
            hunger: craturn::Hunger::$hunger,
        };
    };
}
```

Once expanded, the allocator is global and permanent for the binary.

There is no “stop eating” macro.

<br/>

<br/>

# 🧠 Design Notes

- No locks in allocation paths.
- No heap allocation inside allocator hooks.
- One background eater thread.
- Dense tracking of live allocations.
- Long-lived memory is eaten preferentially.
- Bites are small and localized by default.

This keeps the behavior:
- delayed,
- nondeterministic,
- extremely difficult to trace.

Just like real memory bugs.

<br/>

<br/>

# 🪐 Philosophy

> Saturn does not crash.
> Saturn does not panic.
> Saturn simply eats his son.

`craturn` does the same.

<br/>

<br/>

# 📜 License

MIT OR Apache-2.0  
Choose whichever lets you sleep at night.
