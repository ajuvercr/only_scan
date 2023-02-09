use std::marker::PhantomData;

use diesel::prelude::*;
use r2d2_diesel::ConnectionManager;
use rocket::{
    http::Status,
    request::{self, FromRequest, Outcome},
    Request, State,
};

use diesel::helper_types::{Find, Limit};

use diesel::associations::HasTable;

use diesel::query_builder::*;
use diesel::query_dsl::methods::*;

use crate::util;

pub type Conn = diesel::PgConnection;
pub type Pool = r2d2::Pool<ConnectionManager<Conn>>;

pub struct Repo<U, Table> {
    pub table: Table,
    pub pd: PhantomData<U>,
}

impl<U, T: Table + Copy> Repo<U, T> {
    pub fn new() -> Self
    where
        T: HasTable<Table = T>,
    {
        Self {
            table: T::table(),
            pd: PhantomData,
        }
    }

    pub const fn new_t(table: T) -> Self {
        Self {
            table,
            pd: PhantomData,
        }
    }
}

impl<U, Tab: Table + Copy> Repo<U, Tab> {
    pub fn get_by_id<PK>(&self, id: PK, conn: &mut Conn) -> QueryResult<U>
    where
        Tab: FindDsl<PK>,
        Find<Tab, PK>: LimitDsl,
        Limit<Find<Tab, PK>>: LoadQuery<Conn, U>,
    {
        FindDsl::find(self.table, id).limit(1).get_result(conn)
    }
}

impl<U, Tab: Table + Copy> Repo<U, Tab>
where
    Tab: LoadQuery<Conn, U>,
{
    pub fn get_all(&self, conn: &mut Conn) -> QueryResult<Vec<U>> {
        self.table.load(conn)
    }
}

type Backend = <Conn as Connection>::Backend;

impl<T, Tab> Repo<T, Tab>
where
    Tab: Table + Copy + IntoUpdateTarget + HasTable<Table = Tab>,
    <Tab as QuerySource>::FromClause: QueryFragment<Backend>,
{
    pub fn update_by_id<U, PK, S>(&self, id: PK, update: U, conn: &mut Conn) -> QueryResult<usize>
    where
        Tab: FindDsl<PK, Output = S>,
        S: IntoUpdateTarget + HasTable<Table = Tab>,
        <S as IntoUpdateTarget>::WhereClause: QueryFragment<Backend>,
        U: AsChangeset<Target = Tab>,
        <U as AsChangeset>::Changeset: QueryFragment<Backend>,
    {
        let find: S = self.table.find(id);
        diesel::update(find).set(update).execute(conn)
    }
}

impl<O, Tab> Repo<O, Tab>
where
    Tab: Table + Copy,
    <Tab as QuerySource>::FromClause: QueryFragment<Backend>,
{
    pub fn insert_one<U: 'static, Values>(&self, insert: U, conn: &mut Conn) -> QueryResult<O>
    where
        U: Insertable<Tab, Values = Values>,
        InsertStatement<Tab, Values>: LoadQuery<Conn, O>,
    {
        insert.insert_into(self.table).get_result::<O>(conn)
    }
}

impl<O, Tab> Repo<O, Tab>
where
    Tab: Table + Copy + QueryId,
    <Tab as QuerySource>::FromClause: QueryFragment<Backend>,
{
    pub fn delete_by_id<PK, S>(&self, id: PK, conn: &mut Conn) -> QueryResult<usize>
    where
        Tab: FindDsl<PK, Output = S>,
        S: IntoUpdateTarget + HasTable<Table = Tab>,
        <S as IntoUpdateTarget>::WhereClause: QueryFragment<Backend> + QueryId,
    {
        let find: S = self.table.find(id);
        diesel::delete(find).execute(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::scrum::models::*;
    use crate::pages::scrum::TASK_TABLE;

    #[cfg(test)]
    fn establish_connection() -> Conn {
        let url = "postgres://onlyscan:password@localhost/diesel_demo";
        Conn::establish(&url).unwrap()
    }

    #[test]
    fn test_get() {
        let mut conn = establish_connection();
        let all = TASK_TABLE.get_all(&mut conn);

        println!("{:?}", all);
        assert!(all.is_ok());

        let l = all.unwrap().len();

        let insert: QueryResult<Task> = TASK_TABLE.insert_one(
            TaskNew {
                title: String::from("test"),
            },
            &mut conn,
        );

        println!("{:?}", insert);
        assert!(insert.is_ok());

        let all = TASK_TABLE.get_all(&mut conn);
        println!("{:?}", all);
        assert!(all.is_ok());
        let l2 = all.unwrap().len();
        assert_eq!(l2, l + 1);
    }
}

pub fn init_pool(config: &util::Config) -> Pool {
    let manager = ConnectionManager::new(&config.database_url);
    Pool::new(manager).expect("db pool failed")
}

pub struct DbConn(pub r2d2::PooledConnection<ConnectionManager<Conn>>);

impl std::ops::Deref for DbConn {
    type Target = Conn;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for DbConn {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for DbConn {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let pool: &State<Pool> = req.guard().await.unwrap();
        match pool.get() {
            Ok(conn) => Outcome::Success(DbConn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}
