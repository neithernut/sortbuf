# sortbuf -- data structure for sorting large numbers of items in memory

This library provides types and traits for accumulating a large number of items
in memory and iterating over them in ascending or descending order. It
outperforms `BTree`-based sorting, introduces low memory overhead and allows
insertion of items from multiple threads as well as reacting to allocation
failures without losing data. However, it's sole purpose is sorting and it
provides no other functionality.

## Example

```rust
let mut sortbuf = sortbuf::SortBuf::new();
let mut inserter = sortbuf::Inserter::new(&mut sortbuf);
inserter.insert_items([10, 20, 5, 17]).expect("Failed to insert items");
drop(inserter);
assert!(sortbuf.into_iter().eq([20, 17, 10, 5]));
```

## License

This work is provided under the MIT license. See `LICENSE` for more details.

