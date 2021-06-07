#![recursion_limit = "256"]
#![allow(clippy::expect_fun_call)]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]

//! delicate-scheduler.
// TODO: diesel's io operations are offloaded to `actix_web::web::block`.

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate diesel_migrations;

pub(crate) mod actions;
pub(crate) mod components;
pub(crate) mod db;
pub(crate) mod prelude;

#[macro_use]
pub(crate) mod macros;

use {cfg_mysql_support, cfg_postgres_support};

pub(crate) use prelude::*;

// TODO: return front-end json is small hump patten.
// TODO:

#[actix_web::main]
async fn main() -> AnyResut<()> {
    dotenv().ok();
    db::init();
    let logger = Logger::with_str("info")
        .log_target(LogTarget::File)
        .buffer_and_flush()
        .rotate(
            Criterion::Age(Age::Day),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(10),
        )
        .start()?;

    let delay_timer = DelayTimerBuilder::default().enable_status_report().build();
    let shared_delay_timer = ShareData::new(delay_timer);

    let connection_pool = db::get_connection_pool();
    let shared_connection_pool = ShareData::new(connection_pool);
    let shared_scheduler_meta_info: SharedSchedulerMetaInfo =
        ShareData::new(SchedulerMetaInfo::default());

    let result = HttpServer::new(move || {
        App::new()
            .configure(actions::task::config)
            .configure(actions::user::config)
            .configure(actions::task_log::config)
            .configure(actions::executor_group::config)
            .configure(actions::executor_processor::config)
            .configure(actions::executor_processor_bind::config)
            .configure(actions::data_reports::config)
            .configure(actions::components::config)
            .app_data(shared_delay_timer.clone())
            .app_data(shared_connection_pool.clone())
            .app_data(shared_scheduler_meta_info.clone())
            .wrap(components::session::session_middleware())
            .wrap(MiddlewareLogger::default())
    })
    .bind(
        env::var("SCHEDULER_LISTENING_ADDRESS")
            .expect("Without `SCHEDULER_LISTENING_ADDRESS` set in .env"),
    )?
    .run()
    .await;

    // Finish processing the buffer log first, then process the result.
    logger.shutdown();
    Ok(result?)
}
