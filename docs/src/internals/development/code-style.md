# Code style (WIP)

This article describes some implicit coding style rules used in the project.

## File level


### try to keep size of the source file small

Ideally, the maximum file size is good to have in [300,500] range of lines in total.


### use `*` import to avoid long import lines.

Advantages:
* shorter import
* less lines in total

Disadvantages:
* `it is bad for version control`: it’s harder to track what has been added to the local file namespace.
  Although it is valid, it is not a big issue.
* `it can lead to unnecessary naming collisions`.  Can be solved using aliasing (`alias`/`as` keywords)

__NOTE__: on crate level, [preludes](https://doc.rust-lang.org/beta/reference/names/preludes.html) can be used to have a
collection of names that are automatically brought into scope of every module in a crate.

## Function level


### prefer functional style over imperative

- declarative approach which describes `what to do` rather `how to do`
- more readability as code is naturally grouped.

For example, use list comprehensions over loops:
```rust
let mut sum = 0;
for i in 1..11 {
    sum += i;
}
println!("{}", sum);
```
  vs

```rust
println!("{}", (1..11).fold(0, |a, b| a + b));
```

## Code organization level


### prefer directory/file hierarchy over flat structure


### use variable name shadowing

This helps to reduce hassle in some degree by allowing:
- reusing variable names rather than creating unique ones;
- transforming variables without making them mutable;
- converting type without manually creating two variables of different types (compiler does it automatically)


## Comments


### write comments on public api

It is enforced by `#![warn(missing_docs)]`


### comment non trivial logic, use `NOTE` if necessary


### use `TODO` prefix to signalize about missing implementation


## toolchain


### use code formatter

Cargo formatter can be used:

    cargo fmt

Please note, that the project has some default rules in overridden. Check `.rustfmt.toml` file for details.


### use static code analyzer

Cargo clippy is default tool:

    cargo clippy --all-features -- -D warnings

This command runs clippy tool with the setting which interprets all warning as errors. This should be a default strategy.


### automate some steps on CI

- run unit/component/feature tests
- measure code coverage

