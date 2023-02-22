# nu_plugin_json_path

This [nushell](https://github.com/nushell/nushell) plugin is an attempt to enable the use of `JSONPath`. You can read more about JSONPath in the specification [here](https://www.ietf.org/archive/id/draft-ietf-jsonpath-base-10.html).

## Example Input

This json file is taken from the JSONPath link above.

```json
{
  "store": {
    "book": [
      { "category": "reference",
        "author": "Nigel Rees",
        "title": "Sayings of the Century",
        "price": 8.95
      },
      { "category": "fiction",
        "author": "Evelyn Waugh",
        "title": "Sword of Honour",
        "price": 12.99
      },
      { "category": "fiction",
        "author": "Herman Melville",
        "title": "Moby Dick",
        "isbn": "0-553-21311-3",
        "price": 8.99
      },
      { "category": "fiction",
        "author": "J. R. R. Tolkien",
        "title": "The Lord of the Rings",
        "isbn": "0-395-19395-8",
        "price": 22.99
      }
    ],
    "bicycle": {
      "color": "red",
      "price": 399
    }
  }
}
```
## Syntax and Description

This table shows some example JSONPath syntax and the intended results for testing below.

|#|JSONPath	|Intended result|
|-|-|-|
|1|`$.store.book[*].author`	|the authors of all books in the store|
|2|`$..author`	|all authors|
|3|`$.store.*`	|all things in store, which are some books and a red bicycle|
|4|`$.store..price`	|the prices of everything in the store|
|5|`$..book[2]`	|the third book|
|6|`$..book[-1]`	|the last book in order|
|7|`$..book[0,1]`|the first two books|
|8|`$..book[:2]`	|the first two books|
|9|`$..book[?(@.isbn)]`	|all books with an ISBN number|
|10|`$..book[?(@.price<10)]`	|all books cheaper than 10|
|11|`$..*`	|all member values and array elements |contained in the input value

## Plugin Examples

I saved the above json in a file named test.json, which is also in the repository.

### Example 1

```sh
open test.json | json path '$.store.book[*].author'
```
```
╭───┬──────────────────╮
│ 0 │ Nigel Rees       │
│ 1 │ Evelyn Waugh     │
│ 2 │ Herman Melville  │
│ 3 │ J. R. R. Tolkien │
╰───┴──────────────────╯
```

### Example 2

```sh
open test.json | json path '$..author'
```
```
╭───┬──────────────────╮
│ 0 │ Nigel Rees       │
│ 1 │ Evelyn Waugh     │
│ 2 │ Herman Melville  │
│ 3 │ J. R. R. Tolkien │
╰───┴──────────────────╯
```

### Example 3

```sh
open test.json | json path '$.store.*'
```
```
╭───┬───────────────────╮
│ 0 │ {record 2 fields} │
│ 1 │ [table 4 rows]    │
╰───┴───────────────────╯
```
```sh
open test.json | json path '$.store.*' | get 0
```
```
╭───────┬─────╮
│ color │ red │
│ price │ 399 │
╰───────┴─────╯
```
```sh
open test.json | json path '$.store.*' | get 1
```
```
╭───┬──────────────────┬───────────┬───────┬────────────────────────┬───────────────╮
│ # │      author      │ category  │ price │         title          │     isbn      │
├───┼──────────────────┼───────────┼───────┼────────────────────────┼───────────────┤
│ 0 │ Nigel Rees       │ reference │  8.95 │ Sayings of the Century │            ❎ │
│ 1 │ Evelyn Waugh     │ fiction   │ 12.99 │ Sword of Honour        │            ❎ │
│ 2 │ Herman Melville  │ fiction   │  8.99 │ Moby Dick              │ 0-553-21311-3 │
│ 3 │ J. R. R. Tolkien │ fiction   │ 22.99 │ The Lord of the Rings  │ 0-395-19395-8 │
╰───┴──────────────────┴───────────┴───────┴────────────────────────┴───────────────╯
```
### Example 4

```sh
open test.json | json path '$.store..price'
```
```
╭───┬───────╮
│ 0 │   399 │
│ 1 │  8.95 │
│ 2 │ 12.99 │
│ 3 │  8.99 │
│ 4 │ 22.99 │
╰───┴───────╯
```

### Example 5

```sh
open test.json | json path '$..book[2]'
```
```
╭───┬─────────────────┬──────────┬───────────────┬───────┬───────────╮
│ # │     author      │ category │     isbn      │ price │   title   │
├───┼─────────────────┼──────────┼───────────────┼───────┼───────────┤
│ 0 │ Herman Melville │ fiction  │ 0-553-21311-3 │  8.99 │ Moby Dick │
╰───┴─────────────────┴──────────┴───────────────┴───────┴───────────╯
```

### Example 6

```sh
open test.json | json path '$..book[-1]'
```
```
╭───┬──────────────────┬──────────┬───────────────┬───────┬───────────────────────╮
│ # │      author      │ category │     isbn      │ price │         title         │
├───┼──────────────────┼──────────┼───────────────┼───────┼───────────────────────┤
│ 0 │ J. R. R. Tolkien │ fiction  │ 0-395-19395-8 │ 22.99 │ The Lord of the Rings │
╰───┴──────────────────┴──────────┴───────────────┴───────┴───────────────────────╯
```

### Example 7

```sh
open test.json | json path '$..book[0,1]'
```
```
╭───┬──────────────┬───────────┬───────┬────────────────────────╮
│ # │    author    │ category  │ price │         title          │
├───┼──────────────┼───────────┼───────┼────────────────────────┤
│ 0 │ Nigel Rees   │ reference │  8.95 │ Sayings of the Century │
│ 1 │ Evelyn Waugh │ fiction   │ 12.99 │ Sword of Honour        │
╰───┴──────────────┴───────────┴───────┴────────────────────────╯
```

### Example 8

```sh
open test.json | json path '$..book[:2]'
```
```
╭───┬──────────────┬───────────┬───────┬────────────────────────╮
│ # │    author    │ category  │ price │         title          │
├───┼──────────────┼───────────┼───────┼────────────────────────┤
│ 0 │ Nigel Rees   │ reference │  8.95 │ Sayings of the Century │
│ 1 │ Evelyn Waugh │ fiction   │ 12.99 │ Sword of Honour        │
╰───┴──────────────┴───────────┴───────┴────────────────────────╯
```

### Example 9

```sh
open test.json | json path '$..book[?(@.isbn)]'
```
```
╭───┬──────────────────┬──────────┬───────────────┬───────┬───────────────────────╮
│ # │      author      │ category │     isbn      │ price │         title         │
├───┼──────────────────┼──────────┼───────────────┼───────┼───────────────────────┤
│ 0 │ Herman Melville  │ fiction  │ 0-553-21311-3 │  8.99 │ Moby Dick             │
│ 1 │ J. R. R. Tolkien │ fiction  │ 0-395-19395-8 │ 22.99 │ The Lord of the Rings │
╰───┴──────────────────┴──────────┴───────────────┴───────┴───────────────────────╯
```
### Example 10

```sh
open test.json | json path '$..book[?(@.price<10)]'
```
```
╭───┬─────────────────┬───────────┬───────┬────────────────────────┬───────────────╮
│ # │     author      │ category  │ price │         title          │     isbn      │
├───┼─────────────────┼───────────┼───────┼────────────────────────┼───────────────┤
│ 0 │ Nigel Rees      │ reference │  8.95 │ Sayings of the Century │            ❎ │
│ 1 │ Herman Melville │ fiction   │  8.99 │ Moby Dick              │ 0-553-21311-3 │
╰───┴─────────────────┴───────────┴───────┴────────────────────────┴───────────────╯
```

### Example 11

```sh
open test.json | json path '$..*'
```
```
╭────┬────────────────────────╮
│  0 │ {record 2 fields}      │
│  1 │ {record 2 fields}      │
│  2 │ [table 4 rows]         │
│  3 │ red                    │
│  4 │                    399 │
│  5 │ {record 4 fields}      │
│  6 │ {record 4 fields}      │
│  7 │ {record 5 fields}      │
│  8 │ {record 5 fields}      │
│  9 │ Nigel Rees             │
│ 10 │ reference              │
│ 11 │                   8.95 │
│ 12 │ Sayings of the Century │
│ 13 │ Evelyn Waugh           │
│ 14 │ fiction                │
│ 15 │                  12.99 │
│ 16 │ Sword of Honour        │
│ 17 │ Herman Melville        │
│ 18 │ fiction                │
│ 19 │ 0-553-21311-3          │
│ 20 │                   8.99 │
│ 21 │ Moby Dick              │
│ 22 │ J. R. R. Tolkien       │
│ 23 │ fiction                │
│ 24 │ 0-395-19395-8          │
│ 25 │                  22.99 │
│ 26 │ The Lord of the Rings  │
╰────┴────────────────────────╯
```
# Building, Installing, and Registering

Since this plugin isn't published on crates.io, you will have to have the nushell repository cloned in order to build it.

This is a nushell script provided in [the first issue](https://github.com/fdncred/nu_plugin_json_path/issues/1). In that issue @amtoine explains, "as my repos are located in $env.GIT_REPOS_HOME/<host>/<owner>/<repo>, i had to run the following".

```sh
[nu-plugin nu-protocol] | each {|crate|
    let local = ($env.GIT_REPOS_HOME | path join "github.com" "nushell" "nushell" "crates" $crate)
    cargo add $crate --path $local
}
```

Once the cargo.toml is updated, all you have to do is `cargo install --path .` and then, from within nushell do a `register /path/to/nu_plugin_json_path`.

Good luck!