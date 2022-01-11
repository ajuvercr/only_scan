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
