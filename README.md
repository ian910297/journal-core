- __Add a post:__

  ```bash
  cargo run --bin journal_cli add --title "My New Blog Post" --file "path/to/your/markdown_file.md"
  ```

  cargo run --bin journal_cli add --title "My New Blog Post" --file "./example_posts/202004-simd.md"


- __List posts:__

  ```bash
  cargo run --bin journal_cli list --page 1 --limit 5
  ```

- __Get a post by ID:__

  ```bash
  cargo run --bin journal_cli get --id 1
  ```

- __Update a post:__

  ```bash
  cargo run --bin journal_cli update --id 1 --title "Updated Title" --file "path/to/updated_markdown.md"
  ```

- __Delete a post:__

  ```bash
  cargo run --bin journal_cli delete --id 1
  ```

---------------------------------------------------------------------

- enable server

```shell
cargo run --bin journal-core
```

build cli
```shell
cargo build --bin journal_cli
```
