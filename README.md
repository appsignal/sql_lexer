# SQL lexer

[![Build Status](https://appsignal.semaphoreci.com/badges/sql_lexer/branches/main.svg)](https://appsignal.semaphoreci.com/projects/sql_lexer)
[![Crate](https://img.shields.io/crates/v/sql_lexer.svg)](https://crates.io/crates/sql_lexer)

Rust library to lex and sanitize SQL. To lex a query and write back to a string:

```rust
extern crate sql_lexer;

fn main() {
  let sql = sql_lexer::lex("SELECT * FROM `table`".to_string()).lex();
  println!("{}", sql_lexer::write(sql));
}
```

To sanitize all content from a query so you just get the generic
components:

```rust
extern crate sql_lexer;

fn main() {
  println!("{}", sql_lexer::sanitize_string("SELECT * FROM `table` WHERE id = 1".to_string()));
}
```

This will output:

```sql
SELECT * FROM `table` WHERE id = ?
```

The documentation is available [here](https://docs.rs/sql_lexer).

## Command line

There's a utility included to sanitize a sql query in a file to
facilitate testing:

```
cargo run -- <path-to-file>
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Contribution

Contributions are very welcome. Please make sure that you add a test for any use case you want to add.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
