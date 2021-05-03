#![recursion_limit = "256"]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]

//! delicate-scheduler.
// TODO: diesel's io operations are offloaded to `actix_web::web::block`.

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate diesel_migrations;

mod db;
#[macro_use]
mod macros;

use {cfg_mysql_support, cfg_postgres_support};

use actix_web::web::{self, Data as ShareData};
use actix_web::{post, App, HttpResponse, HttpServer};
use diesel::prelude::*;

use db::model;
use delay_timer::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use dotenv::dotenv;

// use db::schema::posts::dsl::*;

// TODO: return front-end json is small hump patten.

#[post("/api/task/create")]
async fn create_task(
    task: web::Json<model::NewTask>,
    pool: ShareData<db::ConnectionPool>,
) -> HttpResponse {
    use db::schema::task;

    if let Ok(conn) = pool.get() {
        return HttpResponse::Ok().json(Into::<UnifiedResponseMessages<usize>>::into(
            web::block(move || {
                diesel::insert_into(task::table)
                    .values(&*task)
                    .execute(&conn)
            })
            .await,
        ));
    }

    HttpResponse::Ok().json(UnifiedResponseMessages::<()>::error())
}

#[post("/api/task/list")]
async fn show_tasks(pool: ShareData<db::ConnectionPool>) -> HttpResponse {
    use db::schema::task::{self, dsl::*};

    if let Ok(conn) = pool.get() {
        return HttpResponse::Ok().json(Into::<UnifiedResponseMessages<Vec<model::Task>>>::into(
            web::block(move || {
                task.select((
                    task::id,
                    task::name,
                    task::description,
                    task::command,
                    task::frequency,
                    task::cron_expression,
                    task::timeout,
                    task::retry_times,
                    task::retry_interval,
                    task::maximun_parallel_runable_num,
                    task::tag,
                    task::status,
                ))
                .filter(task::status.ne(2))
                .order(id.desc())
                .load::<model::Task>(&conn)
            })
            .await,
        ));
    }

    HttpResponse::Ok().json(UnifiedResponseMessages::<Vec<model::Task>>::error())
}

#[post("/api/task/delete")]
async fn delete_tasks(
    web::Path(task_id): web::Path<i64>,
    pool: ShareData<db::ConnectionPool>,
) -> HttpResponse {
    use db::schema::task::dsl::*;

    if let Ok(conn) = pool.get() {
        return HttpResponse::Ok().json(Into::<UnifiedResponseMessages<usize>>::into(
            web::block(move || diesel::delete(task.find(task_id)).execute(&conn)).await,
        ));
    }

    HttpResponse::Ok().json(UnifiedResponseMessages::<usize>::error())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    db::init();

    let connection_pool = db::get_connection_pool();

    let delay_timer = DelayTimerBuilder::default().enable_status_report().build();

    let shared_delay_timer = ShareData::new(delay_timer);
    let shared_connection_pool = ShareData::new(connection_pool);

    HttpServer::new(move || {
        App::new()
            .service(create_task)
            .app_data(shared_delay_timer.clone())
            .app_data(shared_connection_pool.clone())
    })
    .bind("127.0.0.1:8090")?
    .run()
    .await
}

#[post("/api/task/update")]
async fn update_task(
    web::Json(task_value): web::Json<model::NewTask>,
    pool: ShareData<db::ConnectionPool>,
) -> HttpResponse {
    use db::schema::task;

    if let Ok(conn) = pool.get() {
        return HttpResponse::Ok().json(Into::<UnifiedResponseMessages<usize>>::into(
            web::block(move || {
                diesel::update(task::table.find(task_value.id))
                    .set((
                        task::name.eq(task_value.name),
                        task::description.eq(task_value.description),
                        task::command.eq(task_value.command),
                        task::frequency.eq(task_value.frequency),
                        task::cron_expression.eq(task_value.cron_expression),
                        task::timeout.eq(task_value.timeout),
                        task::retry_times.eq(task_value.retry_times),
                        task::retry_interval.eq(task_value.retry_interval),
                        task::maximun_parallel_runable_num
                            .eq(task_value.maximun_parallel_runable_num),
                        task::tag.eq(task_value.tag),
                    ))
                    .execute(&conn)
            })
            .await,
        ));
    }

    HttpResponse::Ok().json(UnifiedResponseMessages::<usize>::error())
}

// pub fn update_post<'a>(conn: &MysqlConnection, id_num: i64) -> usize {
//     use db::schema::posts;

//     diesel::update(posts::table)
//         .filter(posts::id.eq(id_num))
//         .set(published.eq(1))
//         .execute(conn)
//         .unwrap()
// }

// pub fn update_post_tilte<'a>(conn: &MysqlConnection, id_num: i64) -> usize {
//     diesel::update(posts.find(id_num))
//         .set(title.eq("update"))
//         .execute(conn)
//         .unwrap()
// }

// pub fn delete_post<'a>(conn: &MysqlConnection, id_num: i64) -> usize {
//     diesel::delete(posts.find(id_num)).execute(conn).unwrap()
// }

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub(crate) trait UniformData: Default + Debug + Clone + Serialize {}

impl<T: Default + Debug + Clone + Serialize> UniformData for T {}

/// Uniform public message response format.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UnifiedResponseMessages<T: UniformData> {
    code: i8,
    msg: String,
    data: T,
}

impl<T: UniformData> UnifiedResponseMessages<T> {
    pub(crate) fn success() -> Self {
        UnifiedResponseMessages::default()
    }

    pub(crate) fn success_with_data(data: T) -> Self {
        UnifiedResponseMessages {
            data,
            ..Default::default()
        }
    }

    pub(crate) fn error() -> Self {
        UnifiedResponseMessages {
            code: -1,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub(crate) fn error_with_data(data: T) -> Self {
        let code = -1;
        UnifiedResponseMessages {
            code,
            data,
            ..Default::default()
        }
    }

    pub(crate) fn customized_error_msg(mut self, msg: String) -> Self {
        self.msg = msg;

        self
    }

    #[allow(dead_code)]
    pub(crate) fn customized_error_code(mut self, code: i8) -> Self {
        self.code = code;

        self
    }

    #[allow(dead_code)]
    pub(crate) fn reverse(mut self) -> Self {
        self.code = -1 - self.code;
        self
    }
}

impl<T: UniformData, E: std::error::Error> From<Result<T, E>> for UnifiedResponseMessages<T> {
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(d) => Self::success_with_data(d),
            Err(e) => Self::error().customized_error_msg(e.to_string()),
        }
    }
}
