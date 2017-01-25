# STDIO streams for Tokio

Implementation of tokio_core::io::Io for stdin/stdout streams.

## Example
`Cargo.toml`:
```toml
[dependencies]
futures = "0.1"
tokio-core = "0.1"
tokio-stdio = { git = "https://github.com/smith61/tokio-stdio" }
```

`main.rs`
```rust
extern crate futures;
extern crate tokio_core;
extern crate tokio_stdio;

use futures::{
    Future
};
use tokio_stdio::stdio::{
    Stdio
};
use tokio_core::io::{
    Io
};

pub fn main( ) {
    let ( read, write ) = Stdio::new( 1, 1 ).split( );

    tokio_core::io::copy( read, write ).wait( ).unwrap( );
}
```