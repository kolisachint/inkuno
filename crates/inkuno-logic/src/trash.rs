//! Names in a trash folder: a trashed item keeps its name, numbered when the name is taken. Pure:
//! `taken` says whether a name is already in the folder (the shell asks the disk).

/// The name a trashed item gets in a folder: `name` when free, else `2 name`, `3 name`… (the first
/// free one). Nothing in the folder is replaced.
pub fn name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    (1..)
        .map(|n| match n {
            1 => name.to_string(),
            n => format!("{n} {name}"),
        })
        .find(|candidate| !taken(candidate))
        .expect("the counter never ends")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_free_name_is_kept() {
        assert_eq!(name("Plan.md", |_| false), "Plan.md");
    }

    #[test]
    fn a_taken_name_is_numbered_with_the_first_free_number() {
        let taken = ["Plan.md", "2 Plan.md"];
        assert_eq!(name("Plan.md", |c| taken.contains(&c)), "3 Plan.md");
    }
}
