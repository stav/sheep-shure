use rusqlite::Connection;
use crate::error::AppError;
use crate::models::report::DashboardStats;
use crate::repositories::report_repo;

pub fn get_dashboard_stats(conn: &Connection) -> Result<DashboardStats, AppError> {
    report_repo::get_dashboard_stats(conn)
}
