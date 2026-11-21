# Mixed: a week of project notes

## Summary

This note mixes everything the editor has to draw: **bold**, *italic*, ~~strike~~, ==highlight==, `inline code`, a [link](https://example.com), a wiki link to [[Roadmap]], and a footnote.[^plan]

> [!note] Context
> The release is on the first Monday. Items marked as tasks are tracked below.

### Tasks

- [x] Write the parser tests
- [ ] Check the table wrapping
  - [ ] Narrow window
  - [ ] Wide window
- [ ] Review the code block copy button, which should keep the monospace font

1. Draft the release notes
2. Ask for review
   1. Reviewer one
   2. Reviewer two
3. Publish

### Table

| Task | Owner | Status |
|:-----|:-----:|------:|
| Parser | Ana | **done** |
| Renderer | Ben | *in progress* |

### Code

```rust
let total: u32 = (1..=10).sum();
```

Notes on the formula $\sum_{i=1}^{10} i = 55$ are below.

$$
\sum_{i=1}^{n} i = \frac{n(n+1)}{2}
$$

---

## Reading list

- Book one, with a long description that wraps onto the next line of the editor so the hanging indent can be checked by eye
  - a sub-point under the book
- Book two

![Diagram placeholder](diagram-missing.png)

Closing paragraph with a bare URL https://example.com and a hard break  
on the next line.

[^plan]: The footnote text, which closes the note.
