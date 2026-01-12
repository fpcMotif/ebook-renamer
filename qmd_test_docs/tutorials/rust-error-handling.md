# Rust Error Handling Best Practices

## Introduction

Error handling in Rust uses the `Result<T, E>` type for recoverable errors and `panic!` for unrecoverable errors.

## The Result Type

```rust
fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("Cannot divide by zero".to_string())
    } else {
        Ok(a / b)
    }
}
```

## Using the Question Mark Operator

The `?` operator propagates errors up the call stack:

```rust
use std::fs::File;
use std::io::Read;

fn read_config() -> Result<String, std::io::Error> {
    let mut file = File::open("config.txt")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}
```

## Custom Error Types

```rust
#[derive(Debug)]
enum ConfigError {
    FileNotFound,
    ParseError(String),
    ValidationError(String),
}
```

## Best Practices

1. **Use Result for recoverable errors** - Network failures, file I/O
2. **Use panic! for programming errors** - Array out of bounds, unwrap on None
3. **Implement proper error context** - Use libraries like `anyhow` or `thiserror`
4. **Don't ignore errors** - Always handle Result types explicitly

## Common Patterns

### Early Returns
```rust
fn process_data(data: &str) -> Result<i32, String> {
    if data.is_empty() {
        return Err("Data cannot be empty".to_string());
    }
    // Process data...
    Ok(42)
}
```

### Pattern Matching
```rust
match divide(10.0, 2.0) {
    Ok(result) => println!("Result: {}", result),
    Err(e) => eprintln!("Error: {}", e),
}
```
