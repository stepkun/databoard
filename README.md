# databoard
Implementation of a hierarchical key-value-store with the possibility to do a remapping from a level up to its parent level.

The following restrictions apply:
- Each level can have exactly one parent. 
- There is no remapping down the hierarchy.
- The remapping is evaluated recursively up the levels.

You can do the remapping either
- fully automatic,
- fully manual or 
- do an automatic remapping with manual overrides.

For controlling the access in the hierarchy there are two kinds of special keys:
- keys prefixed with an `@` redirect to the top level `databoard` of the hierarchy.
- keys prefixed with an `_` restrict the access to the curent `databoard`.

## Usage

A standalone databoard:

```rust
use databoard::Databoard;

// instantiation of a default Databoard
let databoard = Databoard::new();
// setting a value returns a `Result` of the previous content, in this case a `None`. 
let old = databoard.set("test", 42).unwrap();
// getting the value may fail, so it returns a `Result`,
let value = databoard.get::<i32>("test").unwrap();
// deleting the value returns a `Result` of the previous content, in this case `42`.
let value = databoard.delete::<i32>("test").unwrap();

```
Hierarchical usage:
```rust
use databoard::{Databoard, Remappings};

let top_level = Databoard::new();
// this creates a databoard with automatic remapping to parent
let level1 = Databoard::with_parent(top_level.clone());
// some remapping rules
let mut remappings = Remappings::default();
remappings.add("test", "{test}");
remappings.add("other_test", "{test}");
// this creates a databoard with manual remapping to parent using the defined remapping rules
let level2 = Databoard::with(Some(level1.clone()), Some(remappings), false);

// sets the value in the top `databoard`
top_level.set("test", 42).unwrap();
// but it can also be accessed from the two other levels
let value1: i32 = level1.get("test").unwrap();
let value2: i32 = level2.get("other_test").unwrap();
```

## License

Licensed with the fair use "NGMC" license, see [license file](https://github.com/stepkun/databoard/blob/main/LICENSE)

## Contribution

Any contribution intentionally submitted for inclusion in the work by you,
shall be licensed with the same "NGMC" license, without any additional terms or conditions.
