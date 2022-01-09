use crud_helper::*;

pub trait Builder<T> {
  fn with(self, t: T) -> Self;
}


#[derive(Crud, Clone)]
struct Story {
  #[id] id: String,
  name: String,
}


#[test]
fn it_works() {
    
    let mut story = Story { id: "test".into(), name: "test".into() };

    let result = 2 + 2;
    assert_eq!(result, 4);
}
