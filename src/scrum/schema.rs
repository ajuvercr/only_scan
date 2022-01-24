table! {
    tasks (id) {
        id -> Int4,
        done -> Bool,
        img -> Nullable<Varchar>,
        title -> Varchar,
        description -> Varchar,
        points -> Int4,
        parent -> Nullable<Varchar>,
        children -> Array<Int4>,
    }
}
