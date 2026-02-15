export interface Client {
  id: string;
  first_name: string;
  last_name: string;
  middle_name?: string;
  dob?: string;
  gender?: string;
  phone?: string;
  phone2?: string;
  email?: string;
  address_line1?: string;
  address_line2?: string;
  city?: string;
  state?: string;
  zip?: string;
  county?: string;
  mbi?: string;
  part_a_date?: string;
  part_b_date?: string;
  orec?: string;
  esrd_status: boolean;
  is_dual_eligible: boolean;
  dual_status_code?: string;
  lis_level?: string;
  medicaid_id?: string;
  lead_source?: string;
  original_effective_date?: string;
  is_active: boolean;
  tags?: string[];
  created_at: string;
  updated_at: string;
}

export interface ClientListItem {
  id: string;
  first_name: string;
  last_name: string;
  dob?: string;
  phone?: string;
  email?: string;
  city?: string;
  state?: string;
  zip?: string;
  mbi?: string;
  is_active: boolean;
  is_dual_eligible: boolean;
}

export interface ClientFilters {
  search?: string;
  carrier_id?: string;
  plan_type_code?: string;
  status_code?: string;
  state?: string;
  zip?: string;
  is_dual_eligible?: boolean;
  is_active?: boolean;
}

export interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
}

export interface Enrollment {
  id: string;
  client_id: string;
  plan_id?: string;
  carrier_id?: string;
  plan_type_code?: string;
  plan_name?: string;
  contract_number?: string;
  pbp_number?: string;
  effective_date?: string;
  termination_date?: string;
  application_date?: string;
  status_code: string;
  enrollment_period?: string;
  disenrollment_reason?: string;
  premium?: number;
  confirmation_number?: string;
  enrollment_source?: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface Carrier {
  id: string;
  name: string;
  short_name?: string;
  is_active: boolean;
}

export interface DashboardStats {
  total_active_clients: number;
  new_this_month: number;
  lost_this_month: number;
  pending_enrollments: number;
  by_plan_type: [string, number][];
  by_carrier: [string, number][];
  by_state: [string, number][];
  monthly_trend: MonthlyTrend[];
}

export interface MonthlyTrend {
  month: string;
  new_clients: number;
  lost_clients: number;
  net: number;
}

export interface EnrollmentListItem {
  id: string;
  client_name: string;
  plan_name?: string;
  carrier_name?: string;
  plan_type?: string;
  status?: string;
  effective_date?: string;
  termination_date?: string;
}
