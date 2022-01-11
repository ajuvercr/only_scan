use crud_helper::*;

pub trait Builder<T> {
    fn with(self, t: T) -> Self;
}

#[derive(Crud, Clone)]
struct Story {
    #[id]
    id: String,
    name: String,
}

#[derive(Crud, Clone)]
struct Wrapper<T> {
    inner: T,
}

#[test]
fn it_works() {
    let mut story = Story {
        id: "test".into(),
        name: "test".into(),
    };

    let builder = Story::builder().with_name("test2".into());

    assert_eq!(story.name, String::from("test"));
    builder.update(&mut story);

    assert_eq!(story.name, String::from("test2"));

    let result = 2 + 2;
    assert_eq!(result, 4);
}

#[test]
fn test_generics() {
    let mut first = Wrapper { inner: 0 };
    let builder = Wrapper::builder().with_inner(42);

    assert_eq!(first.inner, 0);
    builder.update(&mut first);

    assert_eq!(first.inner, 42);
}
