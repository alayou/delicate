use super::prelude::*;

pub(crate) fn route(route: Route) -> Route {
    route.at("/api/user_login_log/list", post(show_user_login_log))
}

#[handler]
async fn show_user_login_log(
    Json(query_params): Json<model::QueryParamsUserLoginLog>,
    pool: Data<&db::ConnectionPool>,
) -> impl IntoResponse {
    if let Ok(conn) = pool.get() {
        return Json(Into::<
            UnifiedResponseMessages<PaginateData<model::FrontEndUserLoginLog>>,
        >::into(
            web::block::<_, _, diesel::result::Error>(move || {
                let query_builder = model::UserLoginLogQueryBuilder::query_all_columns();

                let user_login_log = query_params
                    .clone()
                    .query_filter(query_builder)
                    .paginate(query_params.page)
                    .set_per_page(query_params.per_page)
                    .load::<model::UserLoginLog>(&conn)?;

                let per_page = query_params.per_page;
                let count_builder = model::UserLoginLogQueryBuilder::query_count();
                let count = query_params
                    .query_filter(count_builder)
                    .get_result::<i64>(&conn)?;

                let front_end_user_login_log: Vec<model::FrontEndUserLoginLog> =
                    user_login_log.into_iter().map(|log| log.into()).collect();

                Ok(PaginateData::<model::FrontEndUserLoginLog>::default()
                    .set_data_source(front_end_user_login_log)
                    .set_page_size(per_page)
                    .set_total(count)
                    .set_state_desc::<state::user_login_log::LoginCommand>()
                    .set_state_desc::<state::user_login_log::LoginType>())
            })
            .await,
        ));
    }

    Json(UnifiedResponseMessages::<
        PaginateData<model::FrontEndUserLoginLog>,
    >::error())
}
