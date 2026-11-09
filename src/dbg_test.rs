#[test]
fn dbg_chords() {
    eprintln!(
        "canon ctrl-space = {:?}",
        crate::ui::keys::canon("ctrl-space")
    );
    eprintln!(
        "canon secondary-space = {:?}",
        crate::ui::keys::canon("secondary-space")
    );
    eprintln!(
        "system ctrl-space = {}",
        crate::ui::keys::system("ctrl-space")
    );
    let (c, e, w) = crate::ui::keys::check_json(
        r#"{"version": 9, "bindings": {"a::B": ["a"], "a::C": ["ctrl-space"]}}"#,
    );
    eprintln!("changes={c:?}\nerrors={e:?}\nwarns={w:?}");
}
