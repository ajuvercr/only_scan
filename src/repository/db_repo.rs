use crate::Conn;
use diesel::prelude::*;

use diesel::helper_types::{Find, Limit};

use diesel::associations::HasTable;

use diesel::query_builder::*;
use diesel::query_dsl::methods::*;
pub struct Repo<Table> {
    table: Table,
}

impl<Tab: Table + Copy> Repo<Tab> {
    pub fn get_by_id<U, PK>(&self, id: PK, conn: &mut Conn) -> Option<U>
    where
        Tab: FindDsl<PK>,
        Find<Tab, PK>: LimitDsl,
        Limit<Find<Tab, PK>>: LoadQuery<Conn, U>,
    {
        FindDsl::find(self.table, id).limit(1).get_result(conn).ok()
    }

    pub fn get_all<U>(&self, conn: &mut Conn) -> Option<Vec<U>>
    where
        Tab: LoadQuery<Conn, U>,
    {
        self.table.load(conn).ok()
    }
}

type Backend = <Conn as Connection>::Backend;

impl<Tab> Repo<Tab>
where
    Tab: Table + Copy + IntoUpdateTarget,
    <Tab as QuerySource>::FromClause: QueryFragment<Backend>,
{
    pub fn update_by_id<U, PK>(&self, id: PK, update: U, conn: &mut Conn) -> Option<usize>
    where
        U: AsChangeset<Target = Find<Tab, PK>>,
        <U as AsChangeset>::Changeset: QueryFragment<Backend>,

        Tab: FindDsl<PK, Output = Tab>,
        Find<Tab, PK>: HasTable<Table = Tab>,

        <Find<Tab, PK> as IntoUpdateTarget>::WhereClause: QueryFragment<Backend>,
    {
        let find: Find<Tab, PK> = self.table.find(id);
        diesel::update(find).set(update).execute(conn).ok()
    }
}

impl<Tab> Repo<Tab>
where
    Tab: Table + Copy,
    <Tab as Table>::AllColumns: QueryFragment<Backend>,
    <Tab as QuerySource>::FromClause: QueryFragment<Backend>,
{
    pub fn insert_one<U: 'static, Values>(&self, insert: U, conn: &mut Conn) -> Option<U>
    where
        U: Insertable<Tab, Values = Values>,
        InsertStatement<Tab, Values>: LoadQuery<Conn, U>,
    {
        insert.insert_into(self.table).get_result::<U>(conn).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrum::models::*;

    #[cfg(test)]
    fn establish_connection() -> Conn {
        let url = "postgres://onlyscan:password@localhost/diesel_demo";
        Conn::establish(&url).unwrap()
    }

    #[test]
    fn test_get() {
        let repo = Repo {
            table: tasks::table,
        };

        let mut conn = establish_connection();
        let all = repo.get_all::<Task>(&mut conn);

        assert!(all.is_some());

        let l = all.unwrap().len();

        let insert = repo.insert_one(
            Task {
                id: 0,
                done: false,
                img: None,
                title: String::from("test"),
                description: String::from("test"),
                points: 5,
                parent: None,
                children: Vec::new(),
            },
            &mut conn,
        );

        assert!(insert.is_some());

        let all = repo.get_all::<Task>(&mut conn);
        assert!(all.is_some());
        println!("{:?}", all);
        let l2 = all.unwrap().len();
        assert_eq!(l2, l+1);
    }
}
