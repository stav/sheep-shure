use rusqlite::Connection;
use crate::error::AppError;
use crate::models::report::{DashboardStats, MonthlyTrend};

pub fn get_dashboard_stats(conn: &Connection) -> Result<DashboardStats, AppError> {
    // Total active clients
    let total_active: i64 = conn.query_row(
        "SELECT COUNT(*) FROM clients WHERE is_active = 1",
        [],
        |row| row.get(0),
    )?;

    // New clients this month
    let new_this_month: i64 = conn.query_row(
        "SELECT COUNT(*) FROM clients WHERE is_active = 1 AND created_at >= date('now', 'start of month')",
        [],
        |row| row.get(0),
    )?;

    // Lost clients this month (disenrolled this month)
    let lost_this_month: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT client_id) FROM enrollments WHERE status_code LIKE 'DISENROLLED%' AND updated_at >= date('now', 'start of month')",
        [],
        |row| row.get(0),
    )?;

    // Pending enrollments
    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM enrollments WHERE status_code = 'PENDING' AND is_active = 1",
        [],
        |row| row.get(0),
    )?;

    // By plan type
    let by_plan_type = query_pairs(conn,
        "SELECT COALESCE(e.plan_type_code, 'Unknown'), COUNT(DISTINCT e.client_id) FROM enrollments e WHERE e.status_code = 'ACTIVE' AND e.is_active = 1 GROUP BY e.plan_type_code ORDER BY COUNT(DISTINCT e.client_id) DESC"
    )?;

    // By carrier
    let by_carrier = query_pairs(conn,
        "SELECT COALESCE(c.short_name, c.name, 'Unknown'), COUNT(DISTINCT e.client_id) FROM enrollments e LEFT JOIN carriers c ON e.carrier_id = c.id WHERE e.status_code = 'ACTIVE' AND e.is_active = 1 GROUP BY e.carrier_id ORDER BY COUNT(DISTINCT e.client_id) DESC"
    )?;

    // By state
    let by_state = query_pairs(conn,
        "SELECT COALESCE(cl.state, 'Unknown'), COUNT(*) FROM clients cl WHERE cl.is_active = 1 AND cl.state IS NOT NULL GROUP BY cl.state ORDER BY COUNT(*) DESC LIMIT 15"
    )?;

    // Monthly trend (last 12 months)
    let monthly_trend = get_monthly_trend(conn)?;

    Ok(DashboardStats {
        total_active_clients: total_active,
        new_this_month,
        lost_this_month,
        pending_enrollments: pending,
        by_plan_type,
        by_carrier,
        by_state,
        monthly_trend,
    })
}

fn query_pairs(conn: &Connection, sql: &str) -> Result<Vec<(String, i64)>, AppError> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

fn get_monthly_trend(conn: &Connection) -> Result<Vec<MonthlyTrend>, AppError> {
    let mut trends = Vec::new();

    // Last 12 months
    let mut stmt = conn.prepare(
        "WITH months AS (
            SELECT date('now', 'start of month', '-' || n || ' months') as month_start,
                   date('now', 'start of month', '-' || (n-1) || ' months') as month_end,
                   strftime('%Y-%m', date('now', 'start of month', '-' || n || ' months')) as month_label
            FROM (SELECT 0 as n UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3
                  UNION ALL SELECT 4 UNION ALL SELECT 5 UNION ALL SELECT 6 UNION ALL SELECT 7
                  UNION ALL SELECT 8 UNION ALL SELECT 9 UNION ALL SELECT 10 UNION ALL SELECT 11)
        )
        SELECT m.month_label,
               (SELECT COUNT(*) FROM clients WHERE is_active = 1 AND created_at >= m.month_start AND created_at < m.month_end) as new_count,
               (SELECT COUNT(DISTINCT client_id) FROM enrollments WHERE status_code LIKE 'DISENROLLED%' AND updated_at >= m.month_start AND updated_at < m.month_end) as lost_count
        FROM months m
        ORDER BY m.month_label ASC"
    )?;

    let rows = stmt.query_map([], |row| {
        let new_clients: i64 = row.get(1)?;
        let lost_clients: i64 = row.get(2)?;
        Ok(MonthlyTrend {
            month: row.get(0)?,
            new_clients,
            lost_clients,
            net: new_clients - lost_clients,
        })
    })?;

    for row in rows {
        trends.push(row?);
    }

    Ok(trends)
}
