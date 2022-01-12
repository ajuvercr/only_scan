use crud_helper::*;

#[derive(Builder)]
#[inner(Clone, Debug)]
struct Story<'a> {
    #[id]
    id: &'a str,
    name: &'a str,
}

#[derive(Builder, Clone)]
struct Wrapper<T> {
    inner: T,
}

#[test]
fn it_works() {
    let mut story = Story {
        id: "test",
        name: "test",
    };

    let builder = Story::builder().with_name("test2");
    println!("{:?}", builder);

    assert_eq!(story.name, "test");
    builder.update(&mut story);
    assert_eq!(story.name, "test2");
}

#[test]
fn it_works_rev() {
    let mut story = Story {
        id: "test".into(),
        name: "test".into(),
    };

    let builder = Story::builder().with_name("test2");

    assert_eq!(story.name, "test");
    story.update(builder);
    assert_eq!(story.name, "test2");
}

#[test]
fn test_generics() {
    let mut first = Wrapper { inner: 0 };
    let builder = Wrapper::builder().with_inner(42);

    assert_eq!(first.inner, 0);
    builder.update(&mut first);
    assert_eq!(first.inner, 42);
}
