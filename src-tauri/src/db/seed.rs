use rusqlite::Connection;

use crate::error::AppError;

/// Insert reference/seed data into the database.
/// Uses INSERT OR IGNORE so it is safe to call multiple times.
pub fn seed_data(conn: &Connection) -> Result<(), AppError> {
    tracing::info!("Seeding reference data...");

    seed_plan_types(conn)?;
    seed_enrollment_statuses(conn)?;
    seed_enrollment_periods(conn)?;
    seed_carriers(conn)?;
    seed_states(conn)?;

    tracing::info!("Reference data seeding complete");
    Ok(())
}

fn seed_plan_types(conn: &Connection) -> Result<(), AppError> {
    let sql = "INSERT OR IGNORE INTO plan_types (code, name, description, category) VALUES (?1, ?2, ?3, ?4)";
    let mut stmt = conn.prepare(sql)?;

    let plan_types: &[(&str, &str, &str, &str)] = &[
        ("MA",     "Medicare Advantage",                    "Medicare Part C HMO/PPO plan",                    "ADVANTAGE"),
        ("MAPD",   "Medicare Advantage Prescription Drug",  "Medicare Part C with Part D drug coverage",       "ADVANTAGE"),
        ("PDP",    "Prescription Drug Plan",                "Stand-alone Medicare Part D plan",                "PRESCRIPTION"),
        ("DSNP",   "Dual-Eligible Special Needs Plan",     "For beneficiaries eligible for both Medicare and Medicaid", "ADVANTAGE"),
        ("CSNP",   "Chronic Special Needs Plan",           "For beneficiaries with specific chronic conditions", "ADVANTAGE"),
        ("ISNP",   "Institutional Special Needs Plan",     "For beneficiaries in institutional settings",     "ADVANTAGE"),
        ("MMP",    "Medicare-Medicaid Plan",                "Integrated care for dual-eligible beneficiaries", "ADVANTAGE"),
        ("PACE",   "Programs of All-Inclusive Care",       "Comprehensive care for frail elderly",            "ADVANTAGE"),
        ("MSA",    "Medical Savings Account",               "High-deductible plan with savings account",       "ADVANTAGE"),
        ("PFFS",   "Private Fee-for-Service",              "Plan that pays providers on fee-for-service basis", "ADVANTAGE"),
        ("COST",   "Cost Plan",                             "Medicare cost-reimbursed HMO plan",               "ADVANTAGE"),
        ("MedSupA", "Medigap Plan A",                       "Medicare Supplement Insurance Plan A",             "SUPPLEMENT"),
        ("MedSupB", "Medigap Plan B",                       "Medicare Supplement Insurance Plan B",             "SUPPLEMENT"),
        ("MedSupC", "Medigap Plan C",                       "Medicare Supplement Insurance Plan C",             "SUPPLEMENT"),
        ("MedSupD", "Medigap Plan D",                       "Medicare Supplement Insurance Plan D",             "SUPPLEMENT"),
        ("MedSupF", "Medigap Plan F",                       "Medicare Supplement Insurance Plan F",             "SUPPLEMENT"),
        ("MedSupG", "Medigap Plan G",                       "Medicare Supplement Insurance Plan G",             "SUPPLEMENT"),
        ("MedSupK", "Medigap Plan K",                       "Medicare Supplement Insurance Plan K",             "SUPPLEMENT"),
        ("MedSupL", "Medigap Plan L",                       "Medicare Supplement Insurance Plan L",             "SUPPLEMENT"),
        ("MedSupM", "Medigap Plan M",                       "Medicare Supplement Insurance Plan M",             "SUPPLEMENT"),
        ("MedSupN", "Medigap Plan N",                       "Medicare Supplement Insurance Plan N",             "SUPPLEMENT"),
    ];

    for (code, name, desc, category) in plan_types {
        stmt.execute(rusqlite::params![code, name, desc, category])?;
    }

    Ok(())
}

fn seed_enrollment_statuses(conn: &Connection) -> Result<(), AppError> {
    let sql = "INSERT OR IGNORE INTO enrollment_statuses (code, name, description, is_terminal) VALUES (?1, ?2, ?3, ?4)";
    let mut stmt = conn.prepare(sql)?;

    let statuses: &[(&str, &str, &str, i32)] = &[
        ("ACTIVE",                   "Active",                      "Currently enrolled and active",                0),
        ("PENDING",                  "Pending",                     "Application submitted, awaiting confirmation", 0),
        ("REINSTATED",               "Reinstated",                  "Previously disenrolled, now reinstated",       0),
        ("REJECTED",                 "Rejected",                    "Application was rejected",                     1),
        ("CANCELLED",                "Cancelled",                   "Enrollment was cancelled before effective",    1),
        ("DISENROLLED_VOLUNTARY",    "Disenrolled - Voluntary",    "Member voluntarily disenrolled",               1),
        ("DISENROLLED_INVOLUNTARY",  "Disenrolled - Involuntary",  "Member involuntarily disenrolled",             1),
        ("DISENROLLED_DECEASED",     "Disenrolled - Deceased",     "Member is deceased",                           1),
        ("DISENROLLED_PLAN_TERM",    "Disenrolled - Plan Terminated", "Plan was terminated by CMS or carrier",     1),
        ("DISENROLLED_OTHER_COV",    "Disenrolled - Other Coverage", "Member obtained other creditable coverage",  1),
    ];

    for (code, name, desc, is_terminal) in statuses {
        stmt.execute(rusqlite::params![code, name, desc, is_terminal])?;
    }

    Ok(())
}

fn seed_enrollment_periods(conn: &Connection) -> Result<(), AppError> {
    let sql = "INSERT OR IGNORE INTO enrollment_periods (code, name, description, start_month, start_day, end_month, end_day) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
    let mut stmt = conn.prepare(sql)?;

    let periods: &[(&str, &str, &str, Option<i32>, Option<i32>, Option<i32>, Option<i32>)] = &[
        ("AEP",          "Annual Enrollment Period",          "Annual open enrollment for Medicare Advantage and Part D",  Some(10), Some(15), Some(12), Some(7)),
        ("MA_OEP",       "Medicare Advantage Open Enrollment", "Switch MA plans or drop to Original Medicare",             Some(1),  Some(1),  Some(3),  Some(31)),
        ("IEP",          "Initial Enrollment Period",          "7-month window around 65th birthday",                      None,     None,     None,     None),
        ("ICEP",         "Initial Coverage Election Period",   "When first eligible for Medicare Advantage",               None,     None,     None,     None),
        ("GEP",          "General Enrollment Period",          "Annual enrollment for Medicare Part A and B",               Some(1),  Some(1),  Some(3),  Some(31)),
        ("SEP",          "Special Enrollment Period",          "Triggered by qualifying life events",                       None,     None,     None,     None),
        ("OEPI",         "Open Enrollment Period I",           "Additional open enrollment opportunity",                    None,     None,     None,     None),
        ("FIVESTAR_SEP", "5-Star Special Enrollment Period",   "Switch to a 5-star rated plan at any time",                None,     None,     None,     None),
    ];

    for (code, name, desc, sm, sd, em, ed) in periods {
        stmt.execute(rusqlite::params![code, name, desc, sm, sd, em, ed])?;
    }

    Ok(())
}

fn seed_carriers(conn: &Connection) -> Result<(), AppError> {
    let sql = "INSERT OR IGNORE INTO carriers (id, name, short_name) VALUES (?1, ?2, ?3)";
    let mut stmt = conn.prepare(sql)?;

    let carriers: &[(&str, &str, &str)] = &[
        ("carrier-uhc",       "UnitedHealthcare",      "UHC"),
        ("carrier-humana",    "Humana",                "Humana"),
        ("carrier-aetna",     "Aetna",                 "Aetna"),
        ("carrier-anthem",    "Anthem/Elevance",       "Anthem"),
        ("carrier-wellcare",  "WellCare",              "WellCare"),
        ("carrier-cigna",     "Cigna",                 "Cigna"),
        ("carrier-molina",    "Molina Healthcare",     "Molina"),
        ("carrier-centene",   "Centene",               "Centene"),
        ("carrier-kaiser",    "Kaiser Permanente",     "Kaiser"),
        ("carrier-moo",       "Mutual of Omaha",       "MoO"),
        ("carrier-bcbs",      "Blue Cross Blue Shield", "BCBS"),
        ("carrier-ss",        "SilverScript",          "SilverScript"),
        ("carrier-devoted",   "Devoted Health",        "Devoted"),
        ("carrier-alignment", "Alignment Healthcare",  "Alignment"),
    ];

    for (id, name, short_name) in carriers {
        stmt.execute(rusqlite::params![id, name, short_name])?;
    }

    Ok(())
}

fn seed_states(conn: &Connection) -> Result<(), AppError> {
    let sql = "INSERT OR IGNORE INTO states (code, name) VALUES (?1, ?2)";
    let mut stmt = conn.prepare(sql)?;

    let states: &[(&str, &str)] = &[
        ("AL", "Alabama"),
        ("AK", "Alaska"),
        ("AZ", "Arizona"),
        ("AR", "Arkansas"),
        ("CA", "California"),
        ("CO", "Colorado"),
        ("CT", "Connecticut"),
        ("DE", "Delaware"),
        ("FL", "Florida"),
        ("GA", "Georgia"),
        ("HI", "Hawaii"),
        ("ID", "Idaho"),
        ("IL", "Illinois"),
        ("IN", "Indiana"),
        ("IA", "Iowa"),
        ("KS", "Kansas"),
        ("KY", "Kentucky"),
        ("LA", "Louisiana"),
        ("ME", "Maine"),
        ("MD", "Maryland"),
        ("MA", "Massachusetts"),
        ("MI", "Michigan"),
        ("MN", "Minnesota"),
        ("MS", "Mississippi"),
        ("MO", "Missouri"),
        ("MT", "Montana"),
        ("NE", "Nebraska"),
        ("NV", "Nevada"),
        ("NH", "New Hampshire"),
        ("NJ", "New Jersey"),
        ("NM", "New Mexico"),
        ("NY", "New York"),
        ("NC", "North Carolina"),
        ("ND", "North Dakota"),
        ("OH", "Ohio"),
        ("OK", "Oklahoma"),
        ("OR", "Oregon"),
        ("PA", "Pennsylvania"),
        ("RI", "Rhode Island"),
        ("SC", "South Carolina"),
        ("SD", "South Dakota"),
        ("TN", "Tennessee"),
        ("TX", "Texas"),
        ("UT", "Utah"),
        ("VT", "Vermont"),
        ("VA", "Virginia"),
        ("WA", "Washington"),
        ("WV", "West Virginia"),
        ("WI", "Wisconsin"),
        ("WY", "Wyoming"),
        ("DC", "District of Columbia"),
        ("PR", "Puerto Rico"),
        ("VI", "U.S. Virgin Islands"),
    ];

    for (code, name) in states {
        stmt.execute(rusqlite::params![code, name])?;
    }

    Ok(())
}
