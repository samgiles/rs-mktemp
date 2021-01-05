# mktemp

[![crates.io](https://meritbadge.herokuapp.com/mktemp)](https://crates.io/crates/mktemp)
[![Released API docs](https://docs.rs/mktemp/badge.svg)](https://docs.rs/mktemp)
[![Crates.io](https://img.shields.io/crates/d/mktemp?color=blue)](https://crates.io/crates/mktemp)
[![MPL licensed](https://img.shields.io/github/license/samgiles/rs-mktemp?color=blue)](./LICENSE)
[![CI](https://github.com/samgiles/rs-mktemp/workflows/Stable%20Linux/badge.svg)](https://github.com/samgiles/rs-mktemp/actions?query=workflow%3A%22Stable+Linux%22)

This module provides a simple way of creating temporary files and
directories where their lifetime is defined by the scope they exist in.

Once the variable goes out of scope, the underlying file system resource is removed.

See documentation for full API, and other use cases.

# Example

```
use mktemp::Temp;

{
  let temp_file = Temp::new_file().unwrap();
  let file = try!(fs::File::open(temp_file));
}
// temp_file is cleaned from the fs here
```

# Contributors

Special thanks to our contributors! [Contributors](https://github.com/samgiles/rs-mktemp/graphs/contributors)

# License

MPL v2
